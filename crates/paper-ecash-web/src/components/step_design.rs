use leptos::prelude::*;

use crate::designs::{self, Design};
use crate::models::QrErrorCorrection;

#[component]
pub fn StepDesign(
    designs: RwSignal<Vec<Design>>,
    design_id: RwSignal<String>,
    qr_x_offset: RwSignal<f64>,
    qr_y_offset: RwSignal<f64>,
    qr_size: RwSignal<f64>,
    qr_ec: RwSignal<QrErrorCorrection>,
    on_next: impl Fn() + Send + Sync + 'static,
    on_back: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    let select_design = move |id: &str| {
        design_id.set(id.to_string());
        if let Some(design) = designs::get_design(&designs.get_untracked(), id) {
            qr_x_offset.set(design.qr_x_offset_cm);
            qr_y_offset.set(design.qr_y_offset_cm);
            qr_size.set(design.qr_size_cm);
            qr_ec.set(design.qr_error_correction);
        }
    };

    view! {
        <div class="step">
            <h2>"Design"</h2>
            <p class="step-description">"Choose a note design template."</p>

            {move || {
                let ds = designs.get();
                if ds.is_empty() {
                    view! {
                        <div class="status-message">"Loading designs..."</div>
                    }
                        .into_any()
                } else {
                    view! {
                        <div class="design-grid">
                            {ds
                                .into_iter()
                                .map(|d| {
                                    let id = d.id.clone();
                                    let id2 = d.id.clone();
                                    let name = d.name.clone();
                                    let front_url = d.front_url.clone();
                                    view! {
                                        <div
                                            class=move || {
                                                if design_id.get() == id {
                                                    "design-card selected"
                                                } else {
                                                    "design-card"
                                                }
                                            }
                                            on:click=move |_| select_design(&id2)
                                        >
                                            <img
                                                src=front_url
                                                alt=name.clone()
                                                class="design-thumbnail"
                                            />
                                            <span class="design-name">{name}</span>
                                        </div>
                                    }
                                })
                                .collect::<Vec<_>>()}
                        </div>
                    }
                        .into_any()
                }
            }}

            <div class="qr-settings">
                <h3>"QR Code Settings"</h3>
                <div class="form-row">
                    <div class="form-group">
                        <label>"X Offset (cm)"</label>
                        <input
                            type="number"
                            step="0.1"
                            class="input input-sm"
                            prop:value=move || qr_x_offset.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse() {
                                    qr_x_offset.set(v);
                                }
                            }
                        />
                    </div>
                    <div class="form-group">
                        <label>"Y Offset (cm)"</label>
                        <input
                            type="number"
                            step="0.1"
                            class="input input-sm"
                            prop:value=move || qr_y_offset.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse() {
                                    qr_y_offset.set(v);
                                }
                            }
                        />
                    </div>
                    <div class="form-group">
                        <label>"QR Size (cm)"</label>
                        <input
                            type="number"
                            step="0.1"
                            class="input input-sm"
                            prop:value=move || qr_size.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse() {
                                    qr_size.set(v);
                                }
                            }
                        />
                    </div>
                </div>
            </div>

            <div class="step-actions">
                <button class="btn btn-secondary" on:click=move |_| on_back()>
                    "Back"
                </button>
                <button class="btn btn-primary" on:click=move |_| on_next()>
                    "Next"
                </button>
            </div>
        </div>
    }
}
