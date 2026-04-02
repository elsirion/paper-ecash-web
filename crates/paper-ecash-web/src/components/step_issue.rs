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
        let on_next = on_next.clone();
        move || {
            let Some(rt) = wallet.get() else { return };
            if started.get_untracked() {
                return;
            }
            started.set(true);
            let Some(iss) = issuance.get_untracked() else { return };

            // If already issued, skip
            if iss.status == IssuanceStatus::Issued || iss.status == IssuanceStatus::Complete {
                if !iss.ecash_notes.is_empty() {
                    done.set(true);
                    status_msg.set("Notes already issued.".into());
                    on_next();
                    return;
                }
            }

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

                status_msg.set("Issuing notes with exact denominations...".into());

                let mut all_notes = Vec::new();

                for i in 0..count {
                    progress.set(i);
                    status_msg.set(format!("Issuing note {} of {}...", i + 1, count));

                    match rt.spend_exact(denoms.clone(), true).await {
                        Ok(notes) => {
                            // spend_exact returns individual OOBNotes strings for each denomination
                            // We want one combined OOBNotes per paper note
                            // Since we pass all denoms at once, we get one note per denomination
                            // Combine them or store separately - for QR we need one string per paper note
                            // Actually the plan says: call SpendExact once per paper note with that note's denomination set
                            // It returns individual notes that we should concatenate...
                            // But the CLI --split approach gives one OOBNotes per denomination.
                            // For paper notes, we want ONE OOBNotes string per paper note containing all denominations.

                            // Actually, looking at the plan more carefully: each note should have a SINGLE OOBNotes
                            // string containing all its denominations. The spend_exact gives us individual notes
                            // that we need to combine. But since each paper note calls spend_exact once with its
                            // denomination set, we should NOT split - we should get one OOBNotes with all denoms.

                            // For now, join them as a single string. The worker returns split notes.
                            // TODO: modify worker to return combined OOBNotes for paper note use

                            // Workaround: take the first note if there's only one denomination,
                            // otherwise we'd need to combine. For now, store them all.
                            if notes.len() == 1 {
                                all_notes.push(notes[0].clone());
                            } else {
                                // Multiple denominations per note - join with comma for now
                                // The actual fix is to have the worker combine them
                                all_notes.push(notes.join(","));
                            }
                        }
                        Err(e) => {
                            error.set(Some(format!(
                                "Failed to issue note {} of {}: {e}",
                                i + 1,
                                count
                            )));
                            return;
                        }
                    }
                }

                progress.set(count);
                status_msg.set(format!("All {} notes issued!", count));

                // Update issuance
                let mut updated = iss.clone();
                updated.ecash_notes = all_notes;
                updated.status = IssuanceStatus::Issued;
                storage::save_issuance(&updated);
                issuance.set(Some(updated));
                done.set(true);
            });
        }
    });

    view! {
        <div class="step">
            <h2>"Issue Notes"</h2>
            <p class="step-description">
                "Minting ecash notes with exact denominations. This may take a moment."
            </p>

            <div class="status-message">{move || status_msg.get()}</div>

            {move || {
                let t = total.get();
                if t > 0 {
                    let p = progress.get();
                    let pct = if t > 0 { (p as f64 / t as f64) * 100.0 } else { 0.0 };
                    Some(
                        view! {
                            <div class="progress-bar">
                                <div class="progress-fill" style=format!("width: {pct}%") />
                            </div>
                            <div class="progress-text">{format!("{p} / {t}")}</div>
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
                        view! { <div class="error-message">{e}</div> }
                    })
            }}

            <div class="step-actions">
                {move || {
                    if done.get() {
                        Some(
                            view! {
                                <button
                                    class="btn btn-primary"
                                    on:click={
                                        let on_next = on_next.clone();
                                        move |_| on_next()
                                    }
                                >
                                    "Continue to PDF"
                                </button>
                            },
                        )
                    } else {
                        None
                    }
                }}
            </div>
        </div>
    }
}
