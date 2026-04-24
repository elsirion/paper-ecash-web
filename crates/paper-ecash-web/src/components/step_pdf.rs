use leptos::prelude::*;

use crate::browser;
use crate::designs::{self, Design};
use crate::models::{Issuance, IssuanceStatus};
use crate::pdf;
use crate::qr;
use crate::storage;
use crate::wallet_runtime::WalletRuntime;

#[component]
pub fn StepPdf(
    issuance: RwSignal<Option<Issuance>>,
    designs: RwSignal<Vec<Design>>,
    wallet: RwSignal<Option<WalletRuntime>>,
    on_done: impl Fn() + Send + Sync + Clone + 'static,
    on_back: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let status_msg = RwSignal::new(String::from("Ready to generate PDF."));
    let error = RwSignal::new(Option::<String>::None);
    let generating = RwSignal::new(false);

    // Reclaim state (step 1: reclaim notes back to wallet)
    let reclaim_status = RwSignal::new(Option::<String>::None);
    let reclaim_error = RwSignal::new(Option::<String>::None);
    let reclaiming = RwSignal::new(false);

    // Withdraw state (step 2: send wallet balance out via LN)
    let balance_msat = RwSignal::new(Option::<u64>::None);
    let withdraw_target = RwSignal::new(String::new());
    let withdraw_status = RwSignal::new(Option::<String>::None);
    let withdraw_error = RwSignal::new(Option::<String>::None);
    let withdrawing = RwSignal::new(false);

    // Connect wallet (if not already) and fetch balance on mount
    {
        let started = RwSignal::new(false);
        Effect::new(move || {
            if started.get_untracked() {
                return;
            }
            started.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                let Some(rt) = wallet.get_untracked() else {
                    return;
                };
                // Ensure wallet is connected to this issuance's federation
                if let Some(iss) = issuance.get_untracked() {
                    if let Err(e) = rt
                        .connect(&iss.id, &iss.mnemonic_words, &iss.config.federation_invite)
                        .await
                    {
                        tracing::warn!("Failed to connect wallet: {e}");
                        return;
                    }
                }
                match rt.get_balance().await {
                    Ok(bal) => balance_msat.set(Some(bal)),
                    Err(e) => tracing::warn!("Failed to fetch balance: {e}"),
                }
            });
        });
    }

    let generate = move || {
        let Some(iss) = issuance.get_untracked() else {
            error.set(Some("No issuance data available.".into()));
            return;
        };

        if iss.ecash_notes.is_empty() {
            error.set(Some("No ecash notes to generate PDF from.".into()));
            return;
        }

        generating.set(true);
        error.set(None);
        status_msg.set("Generating QR codes...".into());

        let all_designs = designs.get_untracked();
        wasm_bindgen_futures::spawn_local(async move {
            let design = designs::get_design(&all_designs, &iss.config.design_id);

            // Fetch design images
            let default_front = "https://raw.githubusercontent.com/elsiribot/paper-ecash-note-designs/main/fedi/front.png".to_string();
            let default_back = "https://raw.githubusercontent.com/elsiribot/paper-ecash-note-designs/main/fedi/back.png".to_string();
            let front_url = design.as_ref().map(|d| d.front_url.clone()).unwrap_or(default_front);
            let back_url = design.as_ref().map(|d| d.back_url.clone()).unwrap_or(default_back);

            let front_png = match browser::fetch_image_bytes(&front_url).await {
                Ok(b) => b,
                Err(e) => {
                    error.set(Some(format!("Failed to load front image: {e}")));
                    generating.set(false);
                    return;
                }
            };

            let back_png = match browser::fetch_image_bytes(&back_url).await {
                Ok(b) => b,
                Err(e) => {
                    error.set(Some(format!("Failed to load back image: {e}")));
                    generating.set(false);
                    return;
                }
            };

            // Load QR overlay if needed
            let overlay_png = if let Some(overlay_url) =
                design.as_ref().and_then(|d| d.qr_overlay_url.clone())
            {
                match browser::fetch_image_bytes(&overlay_url).await {
                    Ok(b) => Some(b),
                    Err(_) => None,
                }
            } else {
                None
            };

            // Generate QR codes for each note
            status_msg.set("Generating QR codes...".into());
            let mut qr_pngs = Vec::new();

            for (i, note_str) in iss.ecash_notes.iter().enumerate() {
                status_msg.set(format!(
                    "Generating QR code {} of {}...",
                    i + 1,
                    iss.ecash_notes.len()
                ));

                let qr_png = match qr::generate_qr_png(
                    note_str,
                    iss.config.qr_error_correction,
                    10,
                ) {
                    Ok(png) => png,
                    Err(e) => {
                        error.set(Some(format!("Failed to generate QR code {}: {e}", i + 1)));
                        generating.set(false);
                        return;
                    }
                };

                // Apply overlay if available
                let final_qr = if let Some(overlay) = &overlay_png {
                    match qr::overlay_icon(&qr_png, overlay, 20) {
                        Ok(combined) => combined,
                        Err(_) => qr_png,
                    }
                } else {
                    qr_png
                };

                qr_pngs.push(final_qr);
            }

            // Fetch font and build text config if amount text is set
            let text_config = if let Some(ref text_cfg) = iss.config.amount_text {
                status_msg.set("Fetching font...".into());
                match browser::fetch_image_bytes(&text_cfg.font_url).await {
                    Ok(font_bytes) => {
                        Some(pdf::NoteTextConfig {
                            font_bytes,
                            font_size_pt: text_cfg.font_size_pt as f32,
                            x_offset_cm: text_cfg.x_offset_cm as f32,
                            y_offset_cm: text_cfg.y_offset_cm as f32,
                            width_cm: text_cfg.width_cm as f32,
                            height_cm: text_cfg.height_cm as f32,
                        })
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch font, skipping amount text: {e}");
                        None
                    }
                }
            } else {
                None
            };

            let amount_texts: Vec<String> = if text_config.is_some() {
                let label = iss
                    .config
                    .amount_text
                    .as_ref()
                    .and_then(|t| t.text.clone())
                    .unwrap_or_default();
                vec![label; iss.ecash_notes.len()]
            } else {
                Vec::new()
            };

            // Generate PDF
            status_msg.set("Generating PDF...".into());
            match pdf::generate_pdf(
                &qr_pngs,
                &front_png,
                &back_png,
                iss.config.qr_x_offset_cm,
                iss.config.qr_y_offset_cm,
                iss.config.qr_size_cm,
                true,
                text_config.as_ref().map(|cfg| (cfg, amount_texts.as_slice())),
            ) {
                Ok(pdf_bytes) => {
                    let filename = format!("paper_ecash_{}.pdf", &iss.id[..8]);
                    browser::download_file(&pdf_bytes, &filename, "application/pdf");
                    status_msg.set("PDF downloaded!".into());

                    // Update status to complete
                    let mut updated = iss.clone();
                    updated.status = IssuanceStatus::Complete;
                    storage::save_issuance(&updated);
                    issuance.set(Some(updated));
                }
                Err(e) => {
                    error.set(Some(format!("Failed to generate PDF: {e}")));
                }
            }

            generating.set(false);
        });
    };

    view! {
        <div>
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">"Generate PDF"</h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-6">
                "Generate and download a printable PDF with your paper ecash notes."
            </p>

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
                issuance
                    .get()
                    .map(|iss| {
                        view! {
                            <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-4 mb-6">
                                <div class="flex justify-between text-sm py-1">
                                    <span class="text-gray-500 dark:text-gray-400">"Notes:"</span>
                                    <span class="text-gray-900 dark:text-white">{iss.ecash_notes.len()}</span>
                                </div>
                                <div class="flex justify-between text-sm py-1">
                                    <span class="text-gray-500 dark:text-gray-400">"Per note:"</span>
                                    <span class="text-gray-900 dark:text-white">
                                        {crate::denomination::format_amount_msat(iss.per_note_amount_msat())}
                                    </span>
                                </div>
                                <div class="flex justify-between text-sm py-1">
                                    <span class="text-gray-500 dark:text-gray-400">"Denominations:"</span>
                                    <span class="text-gray-900 dark:text-white">
                                        {iss.config.denominations_msat.iter()
                                            .map(|d| crate::denomination::format_amount_msat(*d))
                                            .collect::<Vec<_>>()
                                            .join(", ")}
                                    </span>
                                </div>
                                <div class="flex justify-between text-sm py-1">
                                    <span class="text-gray-500 dark:text-gray-400">"Design:"</span>
                                    <span class="text-gray-900 dark:text-white">{
                                        designs::get_design(&designs.get_untracked(), &iss.config.design_id)
                                            .map(|d| d.name)
                                            .unwrap_or_else(|| iss.config.design_id.clone())
                                    }</span>
                                </div>
                                <div class="flex justify-between text-sm py-1">
                                    <span class="text-gray-500 dark:text-gray-400">"Pages:"</span>
                                    <span class="text-gray-900 dark:text-white">
                                        {format!(
                                            "{} (front + back)",
                                            (iss.ecash_notes.len() + 3) / 4,
                                        )}
                                    </span>
                                </div>
                                {
                                    let pc = designs::get_design(&designs.get_untracked(), &iss.config.design_id)
                                        .and_then(|d| d.paper_color);
                                    pc.map(|color| view! {
                                        <div class="flex justify-between items-center text-sm py-1">
                                            <span class="text-gray-500 dark:text-gray-400">"Paper color:"</span>
                                            <span class="flex items-center gap-2 text-gray-900 dark:text-white">
                                                <span
                                                    class="inline-block w-4 h-4 rounded border border-gray-300 dark:border-gray-500"
                                                    style=format!("background-color: {color};")
                                                ></span>
                                                {color}
                                            </span>
                                        </div>
                                    })
                                }
                            </div>
                        }
                    })
            }}

            <div class="flex flex-col-reverse sm:flex-row gap-3 sm:justify-end">
                <button
                    class="px-5 py-2.5 text-sm font-medium text-gray-900 bg-white border border-gray-300 rounded-lg hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:focus:ring-gray-700 transition-colors"
                    on:click={
                        let on_back = on_back.clone();
                        move |_| on_back()
                    }
                >
                    "Back"
                </button>
                <button
                    class="px-5 py-2.5 text-sm font-medium text-gray-900 bg-white border border-gray-300 rounded-lg hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:focus:ring-gray-700 transition-colors"
                    on:click={
                        let on_done = on_done.clone();
                        move |_| on_done()
                    }
                >
                    "Done"
                </button>
                <button
                    class="px-5 py-2.5 text-sm font-medium text-gray-900 bg-white border border-gray-300 rounded-lg hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:focus:ring-gray-700 transition-colors"
                    on:click=move |_| {
                        let Some(iss) = issuance.get_untracked() else { return };
                        if iss.ecash_notes.is_empty() { return; }
                        let csv = iss.ecash_notes.join("\n");
                        let filename = format!("paper_ecash_{}.csv", &iss.id[..8]);
                        browser::download_file(csv.as_bytes(), &filename, "text/csv");
                    }
                >
                    "Download Notes"
                </button>
                <button
                    class="px-5 py-2.5 text-sm font-medium text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    disabled=move || generating.get()
                    on:click=move |_| generate()
                >
                    {move || {
                        if generating.get() { "Generating..." } else { "Generate & Download PDF" }
                    }}
                </button>
            </div>

            // ── Step 1: Reclaim notes back to wallet ──
            <div class="mt-8 border border-red-300 dark:border-red-800 rounded-lg p-4 sm:p-6 bg-red-50/50 dark:bg-red-900/10">
                <h3 class="text-base font-semibold text-red-800 dark:text-red-400 mb-1">"Reclaim Issued Notes"</h3>
                <p class="text-sm text-red-700/70 dark:text-red-400/70 mb-4">
                    "Reclaim all issued paper ecash notes back into the wallet. "
                    <strong>"This invalidates every printed note."</strong>
                </p>

                {move || {
                    reclaim_error
                        .get()
                        .map(|e| {
                            view! {
                                <div class="p-3 mb-4 text-sm text-red-800 rounded-lg bg-red-100 dark:bg-red-900/30 dark:text-red-400 border-l-4 border-red-500">{e}</div>
                            }
                        })
                }}

                {move || {
                    reclaim_status
                        .get()
                        .map(|msg| {
                            view! {
                                <div class="p-3 mb-4 text-sm text-blue-800 rounded-lg bg-blue-50 dark:bg-blue-900/30 dark:text-blue-400">{msg}</div>
                            }
                        })
                }}

                <button
                    class="w-full sm:w-auto px-5 py-2.5 text-sm font-medium text-white bg-red-700 rounded-lg hover:bg-red-800 focus:ring-4 focus:ring-red-300 dark:bg-red-600 dark:hover:bg-red-700 dark:focus:ring-red-900 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    disabled=move || {
                        reclaiming.get()
                            || issuance.get().map_or(true, |iss| iss.ecash_notes.is_empty())
                    }
                    on:click=move |_| {
                        let Some(iss) = issuance.get_untracked() else { return };
                        if iss.ecash_notes.is_empty() { return; }
                        let Some(rt) = wallet.get_untracked() else {
                            reclaim_error.set(Some("Wallet not connected.".into()));
                            return;
                        };

                        let notes = iss.ecash_notes.clone();
                        let total = notes.len();
                        reclaiming.set(true);
                        reclaim_error.set(None);
                        reclaim_status.set(Some(format!("Reclaiming note 1 of {total}\u{2026}")));

                        wasm_bindgen_futures::spawn_local(async move {
                            // Expand comma-separated notes from the old
                            // serialization format (pre-single-envelope fix)
                            // into individual OOBNotes strings.
                            let parts: Vec<String> = notes
                                .iter()
                                .flat_map(|s| s.split(',').map(|p| p.trim().to_string()))
                                .filter(|s| !s.is_empty())
                                .collect();
                            let parts_total = parts.len();

                            let mut reclaimed = 0usize;
                            let mut already_spent = 0usize;
                            for (i, note_str) in parts.iter().enumerate() {
                                reclaim_status.set(Some(format!(
                                    "Reclaiming note {} of {parts_total}\u{2026}",
                                    i + 1,
                                )));
                                match rt.reissue_notes(note_str).await {
                                    Ok(()) => reclaimed += 1,
                                    Err(e) => {
                                        tracing::warn!("Note {} already spent or failed: {e:#}", i + 1);
                                        already_spent += 1;
                                    }
                                }
                                // Refresh balance after each note so the user sees funds trickle in
                                if let Ok(bal) = rt.get_balance().await {
                                    balance_msat.set(Some(bal));
                                }
                            }

                            if already_spent == 0 {
                                reclaim_status.set(Some(format!(
                                    "All {parts_total} notes reclaimed to wallet."
                                )));
                            } else if reclaimed == 0 {
                                reclaim_status.set(Some(format!(
                                    "No notes could be reclaimed \u{2014} all {parts_total} had already been redeemed."
                                )));
                            } else {
                                reclaim_status.set(Some(format!(
                                    "{reclaimed} of {parts_total} notes reclaimed. \
                                     {already_spent} had already been redeemed."
                                )));
                            }
                            reclaiming.set(false);
                        });
                    }
                >
                    {move || {
                        if reclaiming.get() { "Reclaiming\u{2026}" } else { "Reclaim All Notes" }
                    }}
                </button>

                <p class="mt-3 text-xs text-red-600/70 dark:text-red-400/50">
                    "Once reclaimed, the paper notes become worthless. This cannot be undone."
                </p>
            </div>

            // ── Step 2: Withdraw wallet balance via Lightning ──
            <div class="mt-4 border border-gray-200 dark:border-gray-700 rounded-lg p-4 sm:p-6 bg-gray-50/50 dark:bg-gray-800/50">
                <h3 class="text-base font-semibold text-gray-900 dark:text-white mb-1">"Withdraw Balance"</h3>
                <p class="text-sm text-gray-500 dark:text-gray-400 mb-4">
                    "Send the wallet balance to a Lightning address or LNURL."
                </p>

                <div class="bg-white dark:bg-gray-800 rounded-lg p-4 mb-4 border border-gray-200 dark:border-gray-700">
                    <div class="flex justify-between items-center text-sm">
                        <span class="text-gray-500 dark:text-gray-400">"Wallet balance:"</span>
                        <span class="font-medium text-gray-900 dark:text-white">
                            {move || match balance_msat.get() {
                                Some(0) => "0 sat (empty)".to_string(),
                                Some(bal) => format!("{} sat", bal / 1000),
                                None => "Loading\u{2026}".to_string(),
                            }}
                        </span>
                    </div>
                </div>

                {move || {
                    withdraw_error
                        .get()
                        .map(|e| {
                            view! {
                                <div class="p-3 mb-4 text-sm text-red-800 rounded-lg bg-red-100 dark:bg-red-900/30 dark:text-red-400 border-l-4 border-red-500">{e}</div>
                            }
                        })
                }}

                {move || {
                    withdraw_status
                        .get()
                        .map(|msg| {
                            view! {
                                <div class="p-3 mb-4 text-sm text-blue-800 rounded-lg bg-blue-50 dark:bg-blue-900/30 dark:text-blue-400">{msg}</div>
                            }
                        })
                }}

                <div class="mb-4">
                    <label class="block mb-1 text-sm font-medium text-gray-900 dark:text-white">"Lightning address or LNURL"</label>
                    <input
                        type="text"
                        class="block w-full p-2.5 text-sm text-gray-900 bg-white rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
                        placeholder="user@domain.com or lnurl1..."
                        prop:value=move || withdraw_target.get()
                        on:input=move |ev| {
                            withdraw_target.set(event_target_value(&ev));
                        }
                    />
                </div>

                <button
                    class="w-full sm:w-auto px-5 py-2.5 text-sm font-medium text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    disabled=move || {
                        withdrawing.get()
                            || withdraw_target.get().trim().is_empty()
                            || balance_msat.get().map_or(true, |b| b == 0)
                    }
                    on:click=move |_| {
                        let target = withdraw_target.get_untracked().trim().to_string();
                        if target.is_empty() { return; }
                        let Some(bal) = balance_msat.get_untracked() else { return };
                        if bal == 0 { return; }
                        let Some(rt) = wallet.get_untracked() else {
                            withdraw_error.set(Some("Wallet not connected.".into()));
                            return;
                        };

                        withdrawing.set(true);
                        withdraw_error.set(None);
                        withdraw_status.set(Some("Resolving Lightning address\u{2026}".into()));

                        wasm_bindgen_futures::spawn_local(async move {
                            match do_withdraw(&rt, &target, bal).await {
                                Ok(()) => {
                                    withdraw_status.set(Some("Withdrawal complete!".into()));
                                }
                                Err(e) => {
                                    withdraw_error.set(Some(format!("{e:#}")));
                                    withdraw_status.set(None);
                                }
                            }
                            // Refresh balance (may be non-zero after internal payment fee savings)
                            if let Ok(bal) = rt.get_balance().await {
                                balance_msat.set(Some(bal));
                            }
                            withdrawing.set(false);
                        });
                    }
                >
                    {move || {
                        if withdrawing.get() { "Sending\u{2026}" } else { "Withdraw" }
                    }}
                </button>
            </div>
        </div>
    }
}

/// Resolve a Lightning address or LNURL to a bolt11 invoice and pay it.
async fn do_withdraw(rt: &WalletRuntime, target: &str, balance_msat: u64) -> anyhow::Result<()> {
    let lnurl_endpoint = if target.contains('@') {
        // Lightning address: user@domain → https://domain/.well-known/lnurlp/user
        let parts: Vec<&str> = target.splitn(2, '@').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            anyhow::bail!("Invalid Lightning address format. Expected user@domain");
        }
        let (user, domain) = (parts[0], parts[1]);
        format!("https://{domain}/.well-known/lnurlp/{user}")
    } else if target.to_lowercase().starts_with("lnurl1") {
        // LNURL bech32
        decode_lnurl(target)?
    } else {
        anyhow::bail!(
            "Unsupported format. Enter a Lightning address (user@domain) or LNURL (lnurl1...)"
        );
    };

    // Fetch gateway fees so we can calculate the max invoice amount that fits
    // within the wallet balance. For internal payments (same federation) the
    // gateway fee won't actually be charged, but we budget for it to be safe.
    let (base_msat, prop_millionths) = rt
        .get_gateway_fees()
        .await
        .unwrap_or((0, 0));

    // invoice_amount + base + invoice_amount * prop / 1_000_000 <= balance
    // invoice_amount * (1_000_000 + prop) / 1_000_000 <= balance - base
    let after_base = balance_msat.saturating_sub(u64::from(base_msat));
    let invoice_amount = if prop_millionths > 0 {
        after_base * 1_000_000 / (1_000_000 + u64::from(prop_millionths))
    } else {
        after_base
    };

    if invoice_amount == 0 {
        anyhow::bail!("Balance too low to cover gateway fees");
    }

    // Step 1: Fetch LNURL-pay metadata
    let meta_json = browser::fetch_json(&lnurl_endpoint).await
        .map_err(|e| anyhow::anyhow!("Failed to fetch LNURL endpoint: {e}"))?;
    let meta: serde_json::Value = serde_json::from_str(&meta_json)
        .map_err(|e| anyhow::anyhow!("Invalid LNURL response: {e}"))?;

    let tag = meta.get("tag").and_then(|v| v.as_str()).unwrap_or("");
    if tag != "payRequest" {
        anyhow::bail!("LNURL endpoint is not a payRequest (got tag={tag:?})");
    }

    let callback = meta
        .get("callback")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("LNURL response missing 'callback' field"))?;
    let min_sendable = meta
        .get("minSendable")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let max_sendable = meta
        .get("maxSendable")
        .and_then(|v| v.as_u64())
        .unwrap_or(u64::MAX);

    if invoice_amount < min_sendable {
        anyhow::bail!(
            "Sendable amount after fees ({} msat) is below the minimum ({} msat)",
            invoice_amount,
            min_sendable,
        );
    }

    // Clamp to maxSendable
    let send_amount = invoice_amount.min(max_sendable);

    // Step 2: Request invoice from callback
    let separator = if callback.contains('?') { '&' } else { '?' };
    let invoice_url = format!("{callback}{separator}amount={send_amount}");
    let invoice_json = browser::fetch_json(&invoice_url).await
        .map_err(|e| anyhow::anyhow!("Failed to fetch invoice: {e}"))?;
    let invoice_resp: serde_json::Value = serde_json::from_str(&invoice_json)
        .map_err(|e| anyhow::anyhow!("Invalid invoice response: {e}"))?;

    if let Some(reason) = invoice_resp.get("reason").and_then(|v| v.as_str()) {
        anyhow::bail!("LNURL error: {reason}");
    }

    let bolt11 = invoice_resp
        .get("pr")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Invoice response missing 'pr' field"))?;

    // Step 3: Pay the invoice
    rt.pay_bolt11(bolt11).await
}

fn decode_lnurl(lnurl: &str) -> anyhow::Result<String> {
    use bech32::Hrp;

    let lnurl_lower = lnurl.to_lowercase();
    let (hrp, data) = bech32::decode(&lnurl_lower)
        .map_err(|e| anyhow::anyhow!("Invalid LNURL bech32: {e}"))?;
    if hrp != Hrp::parse_unchecked("lnurl") {
        anyhow::bail!("Invalid LNURL prefix: expected 'lnurl', got '{hrp}'");
    }
    String::from_utf8(data).map_err(|e| anyhow::anyhow!("LNURL decoded to invalid UTF-8: {e}"))
}
