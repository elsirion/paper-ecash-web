use std::sync::Arc;

use leptos::prelude::*;

use crate::models::{Issuance, IssuanceConfig, IssuanceStatus};
use crate::storage;
use crate::wallet_runtime::{OperationEvent, WalletRuntime};

#[component]
pub fn StepDeposit(
    wallet: RwSignal<Option<WalletRuntime>>,
    issuance: RwSignal<Option<Issuance>>,
    build_config: Arc<dyn Fn() -> IssuanceConfig + Send + Sync>,
    federation_name: RwSignal<String>,
    on_next: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let invoice_str = RwSignal::new(String::new());
    let status_msg = RwSignal::new(String::from("Preparing..."));
    let error = RwSignal::new(Option::<String>::None);
    let paid = RwSignal::new(false);

    // On mount: create or load issuance, connect worker, generate invoice
    let started = RwSignal::new(false);
    Effect::new({
        let build_config = build_config.clone();
        let on_next = on_next.clone();
        move || {
            let Some(rt) = wallet.get() else { return };
            if started.get_untracked() {
                return;
            }
            started.set(true);
            let existing = issuance.get_untracked();
            let build_config = build_config.clone();
            let on_next = on_next.clone();

            wasm_bindgen_futures::spawn_local(async move {
                // Create or use existing issuance
                let iss = match existing {
                    Some(iss) if iss.status != IssuanceStatus::AwaitingDeposit => {
                        // Already funded or beyond
                        paid.set(true);
                        status_msg.set("Already funded.".into());
                        on_next();
                        return;
                    }
                    Some(iss) => iss,
                    None => {
                        let config = build_config();
                        let per_note: u64 = config.denominations_msat.iter().sum();
                        let total = per_note * config.note_count as u64;

                        // Generate 12-word BIP39 mnemonic
                        let mnemonic = fedimint_bip39::Mnemonic::generate_in_with(
                            &mut rand::thread_rng(),
                            fedimint_bip39::Language::English,
                            12,
                        )
                        .expect("Failed to generate mnemonic");
                        let words: String = mnemonic
                            .word_iter()
                            .collect::<Vec<_>>()
                            .join(" ");

                        let id = uuid::Uuid::new_v4().to_string();
                        let new_iss = Issuance {
                            id,
                            created_at: js_sys::Date::now(),
                            label: format!(
                                "{}x {} sats",
                                config.note_count,
                                per_note / 1000,
                            ),
                            config,
                            status: IssuanceStatus::AwaitingDeposit,
                            mnemonic_words: words,
                            ecash_notes: Vec::new(),
                            total_amount_msat: total,
                        };
                        storage::save_issuance(&new_iss);
                        issuance.set(Some(new_iss.clone()));
                        new_iss
                    }
                };

                // Connect to federation with this issuance's mnemonic
                status_msg.set("Connecting to federation...".into());
                if let Err(e) = rt
                    .connect(
                        &iss.id,
                        &iss.mnemonic_words,
                        &iss.config.federation_invite,
                    )
                    .await
                {
                    error.set(Some(format!("Connection failed: {e}")));
                    return;
                }

                // Check if already funded
                match rt.get_balance().await {
                    Ok(balance_msat) if balance_msat >= iss.total_amount_msat => {
                        status_msg.set("Balance sufficient, already funded!".into());
                        let mut updated = iss.clone();
                        updated.status = IssuanceStatus::Funded;
                        storage::save_issuance(&updated);
                        issuance.set(Some(updated));
                        paid.set(true);
                        return;
                    }
                    _ => {}
                }

                // Create invoice
                status_msg.set("Creating Lightning invoice...".into());
                match rt
                    .create_invoice(
                        iss.total_amount_msat,
                        &format!("Paper eCash: {}", iss.label),
                    )
                    .await
                {
                    Ok(resp) => {
                        invoice_str.set(resp.invoice.clone());
                        status_msg.set("Invoice created. Waiting for payment...".into());

                        // Set up payment watcher
                        let on_next = on_next.clone();
                        rt.set_event_listener(Some(Arc::new(move |event| {
                            if let OperationEvent::PaymentReceived { .. } = event {
                                paid.set(true);
                                status_msg.set("Payment received!".into());
                                // Update issuance status
                                if let Some(mut iss) = issuance.get_untracked() {
                                    iss.status = IssuanceStatus::Funded;
                                    storage::save_issuance(&iss);
                                    issuance.set(Some(iss));
                                }
                                on_next();
                            }
                        })));
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to create invoice: {e}")));
                    }
                }
            });
        }
    });

    view! {
        <div>
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">"Deposit"</h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-6">"Pay the Lightning invoice to fund this issuance."</p>

            <div class="text-sm text-gray-500 dark:text-gray-400 mb-4">{move || status_msg.get()}</div>

            {move || {
                error
                    .get()
                    .map(|e| {
                        view! {
                            <div class="p-4 mb-4 text-sm text-red-800 rounded-lg bg-red-50 dark:bg-gray-800 dark:text-red-400 border-l-4 border-red-500">{e}</div>
                        }
                    })
            }}

            {move || {
                let inv = invoice_str.get();
                if paid.get() {
                    None
                } else if inv.is_empty() {
                    Some(
                        view! {
                            <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-8 text-center">
                                <div class="text-sm text-gray-500 dark:text-gray-400 animate-pulse">"Generating invoice..."</div>
                            </div>
                        }
                            .into_any(),
                    )
                } else {
                    Some(
                        view! {
                            <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-4 text-center">
                                <div class="mb-4">
                                    <InvoiceQr invoice=inv.clone() />
                                </div>
                                <div class="flex flex-col items-center gap-3">
                                    <textarea
                                        class="block w-full p-2.5 text-xs text-gray-900 bg-white rounded-lg border border-gray-300 dark:bg-gray-600 dark:border-gray-500 dark:text-white font-mono break-all"
                                        readonly
                                        rows="4"
                                    >
                                        {inv.clone()}
                                    </textarea>
                                    <button
                                        class="px-4 py-2 text-sm font-medium text-gray-900 bg-white border border-gray-300 rounded-lg hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:focus:ring-gray-700 transition-colors"
                                        on:click=move |_| {
                                            let inv = invoice_str.get_untracked();
                                            wasm_bindgen_futures::spawn_local(async move {
                                                let _ = crate::browser::copy_to_clipboard(&inv)
                                                    .await;
                                            });
                                        }
                                    >
                                        "Copy"
                                    </button>
                                </div>
                            </div>
                        }
                            .into_any(),
                    )
                }
            }}

            {move || {
                if let Some(iss) = issuance.get() {
                    Some(
                        view! {
                            <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-4 mt-4">
                                <div class="flex justify-between text-sm py-1">
                                    <span class="text-gray-500 dark:text-gray-400">"Amount:"</span>
                                    <span class="text-gray-900 dark:text-white font-medium">
                                        {format!("{} sats", iss.total_amount_msat / 1000)}
                                    </span>
                                </div>
                            </div>
                        },
                    )
                } else {
                    None
                }
            }}

            {move || {
                if paid.get() {
                    Some(view! {
                        <div class="mt-4 text-sm font-medium text-green-600 dark:text-green-400">"Payment received! Continuing..."</div>
                    })
                } else {
                    None
                }
            }}
        </div>
    }
}

#[component]
fn InvoiceQr(invoice: String) -> impl IntoView {
    let qr_data_uri = {
        match crate::qr::generate_qr_png_white(
            &invoice,
            crate::models::QrErrorCorrection::M,
            8,
        ) {
            Ok(png_bytes) => {
                use wasm_bindgen::JsCast;
                let array = js_sys::Uint8Array::from(&png_bytes[..]);
                let parts = js_sys::Array::new();
                parts.push(&array.buffer());
                let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(
                    &parts,
                    web_sys::BlobPropertyBag::new().type_("image/png"),
                )
                .ok();
                blob.and_then(|b| web_sys::Url::create_object_url_with_blob(&b).ok())
                    .unwrap_or_default()
            }
            Err(_) => String::new(),
        }
    };

    view! {
        <img src=qr_data_uri alt="Invoice QR Code" class="max-w-[256px] w-full mx-auto qr-image" />
    }
}
