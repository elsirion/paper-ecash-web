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
        <div>
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">"Design"</h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-6">"Choose a note design template."</p>

            {move || {
                let ds = designs.get();
                if ds.is_empty() {
                    view! {
                        <div class="text-sm text-gray-500 dark:text-gray-400 text-center py-8">"Loading designs..."</div>
                    }
                        .into_any()
                } else {
                    view! {
                        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-3 mb-6">
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
                                                    "border-2 border-blue-500 dark:border-blue-400 rounded-lg p-2 cursor-pointer text-center bg-blue-50 dark:bg-blue-900/20 transition-all"
                                                } else {
                                                    "border-2 border-gray-200 dark:border-gray-700 rounded-lg p-2 cursor-pointer text-center hover:border-gray-400 dark:hover:border-gray-500 transition-all"
                                                }
                                            }
                                            on:click=move |_| select_design(&id2)
                                        >
                                            <img
                                                src=front_url
                                                alt=name.clone()
                                                class="w-full h-auto rounded mb-1"
                                            />
                                            <span class="text-xs text-gray-600 dark:text-gray-400">{name}</span>
                                        </div>
                                    }
                                })
                                .collect::<Vec<_>>()}
                        </div>
                    }
                        .into_any()
                }
            }}

            <div class="mb-6">
                <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-3">"QR Code Settings"</h3>
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
                    <div>
                        <label class="block mb-1 text-sm text-gray-500 dark:text-gray-400">"X Offset (cm)"</label>
                        <input
                            type="number"
                            step="0.1"
                            class="block w-full p-2 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
                            prop:value=move || qr_x_offset.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse() {
                                    qr_x_offset.set(v);
                                }
                            }
                        />
                    </div>
                    <div>
                        <label class="block mb-1 text-sm text-gray-500 dark:text-gray-400">"Y Offset (cm)"</label>
                        <input
                            type="number"
                            step="0.1"
                            class="block w-full p-2 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
                            prop:value=move || qr_y_offset.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse() {
                                    qr_y_offset.set(v);
                                }
                            }
                        />
                    </div>
                    <div>
                        <label class="block mb-1 text-sm text-gray-500 dark:text-gray-400">"QR Size (cm)"</label>
                        <input
                            type="number"
                            step="0.1"
                            class="block w-full p-2 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
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

            <div class="flex flex-col-reverse sm:flex-row gap-3 sm:justify-end">
                <button
                    class="px-5 py-2.5 text-sm font-medium text-gray-900 bg-white border border-gray-300 rounded-lg hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:focus:ring-gray-700 transition-colors"
                    on:click=move |_| on_back()
                >
                    "Back"
                </button>
                <button
                    class="px-5 py-2.5 text-sm font-medium text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 transition-colors"
                    on:click=move |_| on_next()
                >
                    "Next"
                </button>
            </div>
        </div>
    }
}
