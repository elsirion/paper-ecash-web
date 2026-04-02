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
    on_back: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    let invoice_str = RwSignal::new(String::new());
    let status_msg = RwSignal::new(String::from("Preparing..."));
    let error = RwSignal::new(Option::<String>::None);
    let paid = RwSignal::new(false);

    // On mount: create or load issuance, connect worker, generate invoice
    Effect::new({
        let build_config = build_config.clone();
        let on_next = on_next.clone();
        move || {
            let Some(rt) = wallet.get() else { return };
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
        <div class="step">
            <h2>"Deposit"</h2>
            <p class="step-description">"Pay the Lightning invoice to fund this issuance."</p>

            <div class="status-message">{move || status_msg.get()}</div>

            {move || {
                error
                    .get()
                    .map(|e| {
                        view! { <div class="error-message">{e}</div> }
                    })
            }}

            {move || {
                let inv = invoice_str.get();
                if inv.is_empty() || paid.get() {
                    None
                } else {
                    Some(
                        view! {
                            <div class="invoice-display">
                                <div class="invoice-qr">
                                    <InvoiceQr invoice=inv.clone() />
                                </div>
                                <div class="invoice-text">
                                    <textarea class="input" readonly rows="4">
                                        {inv.clone()}
                                    </textarea>
                                    <button
                                        class="btn btn-sm btn-secondary"
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
                        },
                    )
                }
            }}

            {move || {
                if let Some(iss) = issuance.get() {
                    Some(
                        view! {
                            <div class="deposit-info">
                                <div class="summary-row">
                                    <span>"Amount:"</span>
                                    <span>
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

            <div class="step-actions">
                <button class="btn btn-secondary" on:click=move |_| on_back()>
                    "Back"
                </button>
                {move || {
                    if paid.get() {
                        Some(
                            view! {
                                <button
                                    class="btn btn-primary"
                                    on:click={
                                        let on_next = on_next.clone();
                                        move |_| on_next()
                                    }
                                >
                                    "Continue"
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

#[component]
fn InvoiceQr(invoice: String) -> impl IntoView {
    let qr_data_uri = {
        match crate::qr::generate_qr_png(
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
        <img src=qr_data_uri alt="Invoice QR Code" class="qr-image" />
    }
}
