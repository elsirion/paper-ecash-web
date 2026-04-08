use leptos::prelude::*;

use crate::designs::{self, Design, DesignSource, DEFAULT_DESIGNS_URL};
use crate::fonts;
use crate::models::QrErrorCorrection;
use crate::storage;

const LOCAL_SOURCE: &str = "local:";
const NOTE_WIDTH_CM: f64 = 14.0;
const NOTE_HEIGHT_CM: f64 = 7.0;

/// All designs grouped by source for display.
type SourceGroup = (DesignSource, Vec<Design>);

#[component]
pub fn StepDesign(
    designs: RwSignal<Vec<Design>>,
    design_id: RwSignal<String>,
    qr_x_offset: RwSignal<f64>,
    qr_y_offset: RwSignal<f64>,
    qr_size: RwSignal<f64>,
    qr_ec: RwSignal<QrErrorCorrection>,
    text_sample: RwSignal<String>,
    on_next: impl Fn() + Send + Sync + 'static,
    on_back: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    let source_groups: RwSignal<Vec<SourceGroup>> = RwSignal::new(Vec::new());
    let loading = RwSignal::new(false);
    let show_add_form = RwSignal::new(false);
    let new_source_name = RwSignal::new(String::new());
    let new_source_url = RwSignal::new(String::new());
    let add_error = RwSignal::new(Option::<String>::None);

    // Load all sources (default, local, custom) into source_groups
    let load_all = move || {
        loading.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            let mut groups: Vec<SourceGroup> = Vec::new();

            // Default
            let default_source = DesignSource {
                name: "Default".into(),
                base_url: DEFAULT_DESIGNS_URL.into(),
            };
            match designs::fetch_designs_from(DEFAULT_DESIGNS_URL).await {
                Ok(d) => groups.push((default_source, d)),
                Err(e) => tracing::error!("Failed to fetch default designs: {e:#}"),
            }

            // Local (from design editor, stored in localStorage)
            let local_designs = storage::load_local_designs();
            if !local_designs.is_empty() {
                groups.push((
                    DesignSource {
                        name: "Local (from Design Editor)".into(),
                        base_url: LOCAL_SOURCE.into(),
                    },
                    local_designs,
                ));
            }

            // Custom sources
            for source in storage::load_design_sources() {
                match designs::fetch_designs_from(&source.base_url).await {
                    Ok(d) => groups.push((source, d)),
                    Err(e) => {
                        tracing::warn!("Failed to fetch from {}: {e:#}", source.base_url);
                    }
                }
            }

            let flat: Vec<Design> = groups.iter().flat_map(|(_, ds)| ds.clone()).collect();
            designs.set(flat);
            source_groups.set(groups);
            loading.set(false);
        });
    };

    // Initial load on mount
    load_all();

    let select_design = move |id: &str| {
        design_id.set(id.to_string());
        if let Some(design) = designs::get_design(&designs.get_untracked(), id) {
            qr_x_offset.set(design.qr_x_offset_cm);
            qr_y_offset.set(design.qr_y_offset_cm);
            qr_size.set(design.qr_size_cm);
            qr_ec.set(design.qr_error_correction);
            if let Some(at) = &design.amount_text {
                fonts::inject_font_link(&at.font_family);
            }
        }
    };

    let add_source = move |_| {
        let name = new_source_name.get_untracked().trim().to_string();
        let url = new_source_url.get_untracked().trim().trim_end_matches('/').to_string();
        if name.is_empty() || url.is_empty() {
            add_error.set(Some("Name and URL are required.".into()));
            return;
        }
        let source = DesignSource {
            name: name.clone(),
            base_url: url.clone(),
        };
        add_error.set(None);
        loading.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match designs::fetch_designs_from(&url).await {
                Ok(d) => {
                    // Save source if new
                    let mut saved = storage::load_design_sources();
                    if !saved.iter().any(|s| s.base_url == source.base_url) {
                        saved.push(source.clone());
                        storage::save_design_sources(&saved);
                    }
                    // Append to groups
                    let mut groups = source_groups.get_untracked();
                    groups.push((source, d));
                    let flat: Vec<Design> = groups.iter().flat_map(|(_, ds)| ds.clone()).collect();
                    designs.set(flat);
                    source_groups.set(groups);
                    show_add_form.set(false);
                    new_source_name.set(String::new());
                    new_source_url.set(String::new());
                }
                Err(e) => {
                    add_error.set(Some(format!("Failed to load designs: {e}")));
                }
            }
            loading.set(false);
        });
    };

    let remove_source = move |url: String| {
        // Remove from storage
        let saved: Vec<DesignSource> = storage::load_design_sources()
            .into_iter()
            .filter(|s| s.base_url != url)
            .collect();
        storage::save_design_sources(&saved);
        // Remove from groups
        let mut groups = source_groups.get_untracked();
        groups.retain(|(s, _)| s.base_url != url);
        let flat: Vec<Design> = groups.iter().flat_map(|(_, ds)| ds.clone()).collect();
        designs.set(flat);
        source_groups.set(groups);
    };

    let selected_design = move || -> Option<Design> {
        let id = design_id.get();
        designs::get_design(&designs.get(), &id)
    };

    view! {
        <div>
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">"Design"</h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-4">"Choose a note design template."</p>

            // All source groups
            {move || {
                if loading.get() && source_groups.get().is_empty() {
                    return view! {
                        <div class="text-sm text-gray-500 dark:text-gray-400 text-center py-8">"Loading designs..."</div>
                    }.into_any();
                }
                let groups = source_groups.get();
                if groups.is_empty() {
                    return view! {
                        <div class="text-sm text-gray-500 dark:text-gray-400 text-center py-8">"No designs found."</div>
                    }.into_any();
                }
                view! {
                    <div class="mb-6 space-y-6">
                        {groups.into_iter().map(|(source, ds)| {
                            let source_name = source.name.clone();
                            let source_url = source.base_url.clone();
                            let is_removable = source_url != DEFAULT_DESIGNS_URL && source_url != LOCAL_SOURCE;
                            let url_for_remove = source_url.clone();
                            view! {
                                <div>
                                    <div class="flex items-center justify-between mb-2">
                                        <h3 class="text-sm font-semibold text-gray-900 dark:text-white">{source_name}</h3>
                                        {if is_removable {
                                            view! {
                                                <button
                                                    class="px-2 py-1 text-xs font-medium text-red-700 dark:text-red-400 border border-red-300 dark:border-red-800 hover:bg-red-50 dark:hover:bg-red-900/20 rounded transition-colors"
                                                    on:click=move |_| remove_source(url_for_remove.clone())
                                                >
                                                    "Remove source"
                                                </button>
                                            }.into_any()
                                        } else {
                                            view! { <span></span> }.into_any()
                                        }}
                                    </div>
                                    {if ds.is_empty() {
                                        view! {
                                            <p class="text-xs text-gray-500 dark:text-gray-400 py-2">"No designs in this source."</p>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-3">
                                                {ds.into_iter().map(|d| {
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
                                                            <img src=front_url alt=name.clone() class="w-full h-auto rounded mb-1" />
                                                            <span class="text-xs text-gray-600 dark:text-gray-400">{name}</span>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        }.into_any()
                                    }}
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}

            // Add source button / form
            <div class="mb-6">
                {move || {
                    if show_add_form.get() {
                        view! {
                            <div class="p-3 bg-gray-50 dark:bg-gray-700 rounded-lg border border-gray-200 dark:border-gray-600">
                                <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-3">
                                    <div>
                                        <label class="block mb-1 text-xs text-gray-500 dark:text-gray-400">"Name"</label>
                                        <input
                                            type="text"
                                            placeholder="My Designs"
                                            class="block w-full p-2 text-sm text-gray-900 bg-white rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-600 dark:border-gray-500 dark:text-white"
                                            prop:value=move || new_source_name.get()
                                            on:input=move |ev| new_source_name.set(event_target_value(&ev))
                                        />
                                    </div>
                                    <div>
                                        <label class="block mb-1 text-xs text-gray-500 dark:text-gray-400">"Base URL"</label>
                                        <input
                                            type="text"
                                            placeholder="https://example.com/designs"
                                            class="block w-full p-2 text-sm text-gray-900 bg-white rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-600 dark:border-gray-500 dark:text-white"
                                            prop:value=move || new_source_url.get()
                                            on:input=move |ev| new_source_url.set(event_target_value(&ev))
                                        />
                                    </div>
                                </div>
                                <p class="text-xs text-gray-400 dark:text-gray-500 mb-3">
                                    "URL should point to a directory containing index.json"
                                </p>
                                {move || add_error.get().map(|e| view! {
                                    <div class="text-xs text-red-600 dark:text-red-400 mb-2">{e}</div>
                                })}
                                <div class="flex gap-2 justify-end">
                                    <button
                                        class="px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors"
                                        on:click=move |_| {
                                            show_add_form.set(false);
                                            add_error.set(None);
                                        }
                                    >
                                        "Cancel"
                                    </button>
                                    <button
                                        class="px-3 py-1.5 text-xs font-medium text-white bg-blue-700 rounded-lg hover:bg-blue-800 disabled:opacity-50 transition-colors"
                                        disabled=move || loading.get()
                                        on:click=add_source
                                    >
                                        {move || if loading.get() { "Loading..." } else { "Add Source" }}
                                    </button>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <button
                                class="px-3 py-1.5 text-xs font-medium text-blue-700 dark:text-blue-400 hover:underline"
                                on:click=move |_| show_add_form.set(true)
                            >
                                "+ Add design source"
                            </button>
                        }.into_any()
                    }
                }}
            </div>

            // Amount Text input (only when design has amount_text placement)
            {move || {
                let Some(design) = selected_design() else {
                    return view! { <div></div> }.into_any();
                };
                if design.amount_text.is_none() {
                    return view! { <div></div> }.into_any();
                };
                view! {
                    <div class="mb-6">
                        <label class="block mb-1 text-sm font-medium text-gray-900 dark:text-white">"Amount Text"</label>
                        <input
                            type="text"
                            placeholder="e.g. 1000 sats"
                            class="block w-full p-2.5 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            prop:value=move || text_sample.get()
                            on:input=move |ev| text_sample.set(event_target_value(&ev))
                        />
                        <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                            "This text will be printed on each note at the position defined by the design."
                        </p>
                    </div>
                }.into_any()
            }}

            // Preview pane
            {move || {
                let Some(design) = selected_design() else {
                    return view! { <div></div> }.into_any();
                };
                let front = design.front_url.clone();
                let amount_text = design.amount_text.clone();
                let qr_x = design.qr_x_offset_cm;
                let qr_y = design.qr_y_offset_cm;
                let qr_sz = design.qr_size_cm;
                let qr_left = qr_x / NOTE_WIDTH_CM * 100.0;
                let qr_top = qr_y / NOTE_HEIGHT_CM * 100.0;
                let qr_w = qr_sz / NOTE_WIDTH_CM * 100.0;
                let qr_h = qr_sz / NOTE_HEIGHT_CM * 100.0;
                view! {
                    <div class="mb-6">
                        <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-3">"Preview"</h3>
                        <div class="border border-gray-300 dark:border-gray-600 rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-900">
                            <div
                                class="relative w-full"
                                style="aspect-ratio: 2 / 1; container-type: size;"
                            >
                                <img
                                    src=front
                                    class="absolute inset-0 w-full h-full object-fill pointer-events-none"
                                    draggable="false"
                                />
                                <div
                                    class="absolute border border-dashed border-gray-400 dark:border-gray-500 pointer-events-none"
                                    style=format!(
                                        "left: {:.2}%; top: {:.2}%; width: {:.2}%; height: {:.2}%;",
                                        qr_left, qr_top, qr_w, qr_h
                                    )
                                ></div>
                                {amount_text.as_ref().map(|at| {
                                    let font = at.font_family.clone();
                                    let color = at.color_hex.clone();
                                    let x_pct = at.x_offset_cm / NOTE_WIDTH_CM * 100.0;
                                    let y_pct = at.y_offset_cm / NOTE_HEIGHT_CM * 100.0;
                                    let w_pct = at.width_cm / NOTE_WIDTH_CM * 100.0;
                                    let h_pct = at.height_cm / NOTE_HEIGHT_CM * 100.0;
                                    let fs = h_pct * 0.75;
                                    let sample = text_sample.get();
                                    let display = if sample.trim().is_empty() {
                                        "1000 sats".to_string()
                                    } else {
                                        sample
                                    };
                                    view! {
                                        <div
                                            class="absolute pointer-events-none flex items-center justify-center"
                                            style=format!(
                                                "left: {x_pct:.2}%; top: {y_pct:.2}%; width: {w_pct:.2}%; height: {h_pct:.2}%;",
                                            )
                                        >
                                            <span
                                                class="whitespace-nowrap"
                                                style=format!(
                                                    "font-family: '{font}', sans-serif; font-size: {fs:.3}cqh; color: {color}; line-height: 1;",
                                                )
                                            >
                                                {display}
                                            </span>
                                        </div>
                                    }
                                })}
                            </div>
                        </div>
                    </div>
                }.into_any()
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
                    class="px-5 py-2.5 text-sm font-medium text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    disabled=move || {
                        let design = selected_design();
                        match design {
                            None => true,
                            Some(d) => {
                                d.amount_text.is_some() && text_sample.get().trim().is_empty()
                            }
                        }
                    }
                    on:click=move |_| on_next()
                >
                    "Next"
                </button>
            </div>
        </div>
    }
}

