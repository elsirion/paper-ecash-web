use leptos::prelude::*;

use crate::models::{Issuance, IssuanceStatus};
use crate::storage;
use crate::wallet_runtime::WalletRuntime;

#[component]
pub fn StepIssue(
    wallet: RwSignal<Option<WalletRuntime>>,
    issuance: RwSignal<Option<Issuance>>,
    on_next: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let progress = RwSignal::new(0u32);
    let total = RwSignal::new(0u32);
    let status_msg = RwSignal::new(String::from("Preparing to issue notes..."));
    let error = RwSignal::new(Option::<String>::None);
    let done = RwSignal::new(false);

    let started = RwSignal::new(false);
    Effect::new({
        let on_next = std::sync::Arc::new(on_next);
        move || {
            let on_next = on_next.clone();
            let Some(rt) = wallet.get() else { return };
            if started.get_untracked() {
                return;
            }
            started.set(true);
            let Some(iss) = issuance.get_untracked() else { return };

            let count = iss.config.note_count;
            let denoms = iss.config.denominations_msat.clone();
            total.set(count);

            wasm_bindgen_futures::spawn_local(async move {
                // Ensure connected
                if let Err(e) = rt
                    .connect(&iss.id, &iss.mnemonic_words, &iss.config.federation_invite)
                    .await
                {
                    error.set(Some(format!("Connection failed: {e}")));
                    return;
                }

                // Recover any notes already minted from the operation log.
                // This handles the case where the app was reloaded mid-issuance.
                status_msg.set("Checking for previously issued notes...".into());
                let mut all_notes: Vec<String> = match rt.recover_issued_notes().await {
                    Ok(notes) => notes,
                    Err(e) => {
                        tracing::warn!("Failed to recover notes from oplog: {e}");
                        Vec::new()
                    }
                };

                let already_minted = all_notes.len() as u32;
                if already_minted > 0 {
                    status_msg.set(format!(
                        "Recovered {} previously issued notes.",
                        already_minted
                    ));
                    progress.set(already_minted);
                }

                // If we already have all notes, skip minting
                if already_minted >= count {
                    all_notes.truncate(count as usize);
                    progress.set(count);
                    status_msg.set(format!("All {} notes issued!", count));
                    let mut updated = iss.clone();
                    updated.ecash_notes = all_notes;
                    updated.status = IssuanceStatus::Issued;
                    storage::save_issuance(&updated);
                    issuance.set(Some(updated));
                    done.set(true);
                    on_next();
                    return;
                }

                // Wait for balance to be available
                let remaining = count - already_minted;
                status_msg.set("Checking balance...".into());
                let per_note_msat: u64 = denoms.iter().sum();
                let required = per_note_msat * remaining as u64;
                let mut attempts = 0u32;
                loop {
                    match rt.get_balance().await {
                        Ok(balance) if balance >= required => break,
                        Ok(balance) => {
                            if attempts >= 60 {
                                error.set(Some(format!(
                                    "Insufficient balance after waiting: have {} msat, need {} msat",
                                    balance, required
                                )));
                                return;
                            }
                            attempts += 1;
                            status_msg.set(format!(
                                "Waiting for balance... ({} / {} msat)",
                                balance, required
                            ));
                            gloo_timers::future::TimeoutFuture::new(2_000).await;
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to check balance: {e}")));
                            return;
                        }
                    }
                }

                status_msg.set("Issuing notes with exact denominations...".into());

                for i in already_minted..count {
                    progress.set(i);
                    status_msg.set(format!("Issuing note {} of {}...", i + 1, count));

                    match rt.spend_exact(denoms.clone()).await {
                        Ok(note) => {
                            all_notes.push(note);
                        }
                        Err(e) => {
                            tracing::warn!("spend_exact failed at note {}/{count}: {e:#}", i + 1);
                            // Likely ran out of balance due to federation fees.
                            // Continue with the notes we already have.
                            if all_notes.is_empty() {
                                error.set(Some(format!(
                                    "Failed to issue any notes: {e}"
                                )));
                                return;
                            }
                            error.set(Some(format!(
                                "Could only issue {} of {count} notes \
                                 (likely due to federation fees eating into the balance). \
                                 The remaining balance can be reclaimed on the download page.",
                                all_notes.len(),
                            )));
                            break;
                        }
                    }
                }

                let issued = all_notes.len() as u32;
                progress.set(issued);
                if issued == count {
                    status_msg.set(format!("All {count} notes issued!"));
                } else {
                    status_msg.set(format!("{issued} of {count} notes issued."));
                }

                // Update issuance
                let mut updated = iss.clone();
                updated.ecash_notes = all_notes;
                updated.status = IssuanceStatus::Issued;
                storage::save_issuance(&updated);
                issuance.set(Some(updated));
                done.set(true);

                // Auto-progress to next step
                on_next();
            });
        }
    });

    view! {
        <div>
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">"Issue Notes"</h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-6">
                "Minting ecash notes with exact denominations. This may take a moment."
            </p>

            <div class="text-sm text-gray-500 dark:text-gray-400 mb-4">{move || status_msg.get()}</div>

            {move || {
                let t = total.get();
                if t > 0 {
                    let p = progress.get();
                    let pct = if t > 0 { (p as f64 / t as f64) * 100.0 } else { 0.0 };
                    Some(
                        view! {
                            <div class="w-full bg-gray-200 rounded-full h-2.5 dark:bg-gray-700 my-4">
                                <div
                                    class="bg-blue-600 h-2.5 rounded-full transition-all duration-300"
                                    style=format!("width: {pct}%")
                                />
                            </div>
                            <div class="text-center text-sm text-gray-500 dark:text-gray-400">{format!("{p} / {t}")}</div>
                        },
                    )
                } else {
                    None
                }
            }}

            {move || {
                error
                    .get()
                    .map(|e| {
                        view! {
                            <div class="p-4 mt-4 text-sm text-red-800 rounded-lg bg-red-50 dark:bg-gray-800 dark:text-red-400 border-l-4 border-red-500">{e}</div>
                        }
                    })
            }}

            {move || {
                if done.get() {
                    Some(view! {
                        <div class="mt-4 text-sm font-medium text-green-600 dark:text-green-400">"All notes issued! Generating PDF..."</div>
                    })
                } else {
                    None
                }
            }}
        </div>
    }
}
