use leptos::prelude::*;

use crate::designs::{self, Design, DesignSource, DEFAULT_DESIGNS_URL};
use crate::models::QrErrorCorrection;
use crate::storage;

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
    let default_source = DesignSource {
        name: "Default".into(),
        base_url: DEFAULT_DESIGNS_URL.into(),
    };

    let custom_sources = RwSignal::new(storage::load_design_sources());
    let selected_url = RwSignal::new(DEFAULT_DESIGNS_URL.to_string());
    let show_add_form = RwSignal::new(false);
    let new_source_name = RwSignal::new(String::new());
    let new_source_url = RwSignal::new(String::new());
    let add_error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);

    // Load designs from a source URL
    let load_source = move |url: String| {
        loading.set(true);
        selected_url.set(url.clone());
        wasm_bindgen_futures::spawn_local(async move {
            match designs::fetch_designs_from(&url).await {
                Ok(d) => {
                    designs.set(d);
                    design_id.set(String::new());
                }
                Err(e) => tracing::error!("Failed to fetch designs from {url}: {e:#}"),
            }
            loading.set(false);
        });
    };

    let select_design = move |id: &str| {
        design_id.set(id.to_string());
        if let Some(design) = designs::get_design(&designs.get_untracked(), id) {
            qr_x_offset.set(design.qr_x_offset_cm);
            qr_y_offset.set(design.qr_y_offset_cm);
            qr_size.set(design.qr_size_cm);
            qr_ec.set(design.qr_error_correction);
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
        // Test the source by loading it
        add_error.set(None);
        loading.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match designs::fetch_designs_from(&url).await {
                Ok(d) => {
                    // Save source
                    let mut sources = custom_sources.get_untracked();
                    if !sources.iter().any(|s| s.base_url == source.base_url) {
                        sources.push(source);
                        storage::save_design_sources(&sources);
                        custom_sources.set(sources);
                    }
                    // Switch to it
                    selected_url.set(url);
                    designs.set(d);
                    design_id.set(String::new());
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
        let mut sources = custom_sources.get_untracked();
        sources.retain(|s| s.base_url != url);
        storage::save_design_sources(&sources);
        custom_sources.set(sources);
        // If we removed the active source, switch back to default
        if selected_url.get_untracked() == url {
            let load = load_source.clone();
            load(DEFAULT_DESIGNS_URL.to_string());
        }
    };

    view! {
        <div>
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">"Design"</h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-4">"Choose a note design template."</p>

            // Source selector
            <div class="mb-6">
                <label class="block mb-1 text-sm font-medium text-gray-900 dark:text-white">"Design Source"</label>
                <div class="flex gap-2">
                    <select
                        class="block w-full p-2.5 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        on:change={
                            let load = load_source.clone();
                            move |ev| {
                                let url = event_target_value(&ev);
                                load(url);
                            }
                        }
                        prop:value=move || selected_url.get()
                    >
                        <option value=DEFAULT_DESIGNS_URL>"Default"</option>
                        {move || {
                            custom_sources.get().into_iter().map(|s| {
                                let url = s.base_url.clone();
                                let name = s.name.clone();
                                view! {
                                    <option value=url>{name}</option>
                                }
                            }).collect_view()
                        }}
                    </select>
                    // Remove button (only for custom sources)
                    {move || {
                        let url = selected_url.get();
                        if url != DEFAULT_DESIGNS_URL {
                            let url2 = url.clone();
                            view! {
                                <button
                                    class="px-3 py-2 text-xs font-medium text-red-700 dark:text-red-400 border border-red-300 dark:border-red-800 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors whitespace-nowrap"
                                    on:click=move |_| remove_source(url2.clone())
                                >
                                    "Remove"
                                </button>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                </div>

                // Add source button / form
                {move || {
                    if show_add_form.get() {
                        view! {
                            <div class="mt-3 p-3 bg-gray-50 dark:bg-gray-700 rounded-lg border border-gray-200 dark:border-gray-600">
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
                                class="mt-2 px-3 py-1.5 text-xs font-medium text-blue-700 dark:text-blue-400 hover:underline"
                                on:click=move |_| show_add_form.set(true)
                            >
                                "+ Add design source"
                            </button>
                        }.into_any()
                    }
                }}
            </div>

            {move || {
                let ds = designs.get();
                if loading.get() {
                    view! {
                        <div class="text-sm text-gray-500 dark:text-gray-400 text-center py-8">"Loading designs..."</div>
                    }
                        .into_any()
                } else if ds.is_empty() {
                    view! {
                        <div class="text-sm text-gray-500 dark:text-gray-400 text-center py-8">"No designs found."</div>
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
