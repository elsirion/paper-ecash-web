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
        <div class="step">
            <h2>"Generate PDF"</h2>
            <p class="step-description">
                "Generate and download a printable PDF with your paper ecash notes."
            </p>

            <div class="status-message">{move || status_msg.get()}</div>

            {move || {
                error
                    .get()
                    .map(|e| {
                        view! { <div class="error-message">{e}</div> }
                    })
            }}

            {move || {
                issuance
                    .get()
                    .map(|iss| {
                        view! {
                            <div class="pdf-summary">
                                <div class="summary-row">
                                    <span>"Notes:"</span>
                                    <span>{iss.ecash_notes.len()}</span>
                                </div>
                                <div class="summary-row">
                                    <span>"Design:"</span>
                                    <span>{iss.config.design_id.clone()}</span>
                                </div>
                                <div class="summary-row">
                                    <span>"Pages:"</span>
                                    <span>
                                        {format!(
                                            "{} (front + back)",
                                            (iss.ecash_notes.len() + 3) / 4,
                                        )}
                                    </span>
                                </div>
                            </div>
                        }
                    })
            }}

            <div class="step-actions">
                <button
                    class="btn btn-primary"
                    disabled=move || generating.get()
                    on:click=move |_| generate()
                >
                    {move || {
                        if generating.get() { "Generating..." } else { "Generate & Download PDF" }
                    }}
                </button>
                <button
                    class="btn btn-secondary"
                    on:click={
                        let on_done = on_done.clone();
                        move |_| on_done()
                    }
                >
                    "Done"
                </button>
            </div>
        </div>
    }
}
