use leptos::prelude::*;

use crate::browser;
use crate::designs::{self, Design};
use crate::models::{Issuance, IssuanceStatus};
use crate::pdf;
use crate::qr;
use crate::storage;

#[component]
pub fn StepPdf(
    issuance: RwSignal<Option<Issuance>>,
    designs: RwSignal<Vec<Design>>,
    on_done: impl Fn() + Send + Sync + Clone + 'static,
    on_back: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let status_msg = RwSignal::new(String::from("Ready to generate PDF."));
    let error = RwSignal::new(Option::<String>::None);
    let generating = RwSignal::new(false);

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
                    class="px-5 py-2.5 text-sm font-medium text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    disabled=move || generating.get()
                    on:click=move |_| generate()
                >
                    {move || {
                        if generating.get() { "Generating..." } else { "Generate & Download PDF" }
                    }}
                </button>
            </div>
        </div>
    }
}
