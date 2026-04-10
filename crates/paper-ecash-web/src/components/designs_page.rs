use leptos::prelude::*;

use crate::browser;
use crate::designs::{self, Design, DesignSource, DEFAULT_DESIGNS_URL};
use crate::fonts;
use crate::models::QrErrorCorrection;
use crate::qr;
use crate::storage;

const LOCAL_SOURCE: &str = "local:";
const NOTE_WIDTH_CM: f64 = 14.0;
const NOTE_HEIGHT_CM: f64 = 7.0;

fn make_sample_qr_url(ec: QrErrorCorrection) -> String {
    match qr::generate_qr_png_white(qr::SAMPLE_QR_DATA, ec, 4) {
        Ok(png) => browser::png_object_url(&png),
        Err(_) => String::new(),
    }
}

type SourceGroup = (DesignSource, Vec<Design>);

#[component]
pub fn DesignsPage(
    on_new: impl Fn() + Send + Sync + Clone + 'static,
    on_edit: impl Fn(Design) + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let source_groups: RwSignal<Vec<SourceGroup>> = RwSignal::new(Vec::new());
    let loading = RwSignal::new(false);
    let show_add_form = RwSignal::new(false);
    let new_source_name = RwSignal::new(String::new());
    let new_source_url = RwSignal::new(String::new());
    let add_error = RwSignal::new(Option::<String>::None);
    let preview_design: RwSignal<Option<Design>> = RwSignal::new(None);
    let preview_text = RwSignal::new("1048".to_string());
    let paper_color = RwSignal::new("#ffffff".to_string());

    loading.set(true);
    wasm_bindgen_futures::spawn_local(async move {
        let mut groups: Vec<SourceGroup> = Vec::new();

        let default_source = DesignSource {
            name: "Default".into(),
            base_url: DEFAULT_DESIGNS_URL.into(),
        };
        if let Ok(d) = designs::fetch_designs_from(DEFAULT_DESIGNS_URL).await {
            groups.push((default_source, d));
        }

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

        for source in storage::load_design_sources() {
            if let Ok(d) = designs::fetch_designs_from(&source.base_url).await {
                groups.push((source, d));
            }
        }

        source_groups.set(groups);
        loading.set(false);
    });

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
                    let mut saved = storage::load_design_sources();
                    if !saved.iter().any(|s| s.base_url == source.base_url) {
                        saved.push(source.clone());
                        storage::save_design_sources(&saved);
                    }
                    let mut groups = source_groups.get_untracked();
                    groups.push((source, d));
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
        let saved: Vec<DesignSource> = storage::load_design_sources()
            .into_iter()
            .filter(|s| s.base_url != url)
            .collect();
        storage::save_design_sources(&saved);
        let mut groups = source_groups.get_untracked();
        groups.retain(|(s, _)| s.base_url != url);
        source_groups.set(groups);
    };

    view! {
        <div>
            <div class="flex items-center justify-between mb-6">
                <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"Designs"</h2>
                <div class="flex gap-2">
                    <button
                        class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                        on:click=move |_| show_add_form.set(!show_add_form.get_untracked())
                    >
                        "Add Source"
                    </button>
                    <button
                        class="px-4 py-2 text-sm font-medium text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 transition-colors"
                        on:click={
                            let on_new = on_new.clone();
                            move |_| on_new()
                        }
                    >
                        "Create New Design"
                    </button>
                </div>
            </div>

            // Add source form
            {move || {
                if !show_add_form.get() {
                    return view! { <div></div> }.into_any();
                }
                view! {
                    <div class="p-4 mb-6 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                        <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-3">"Add Design Source"</h3>
                        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-3">
                            <div>
                                <label class="block mb-1 text-xs text-gray-500 dark:text-gray-400">"Name"</label>
                                <input
                                    type="text"
                                    placeholder="My Designs"
                                    class="block w-full p-2 text-sm text-gray-900 bg-white rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                    prop:value=move || new_source_name.get()
                                    on:input=move |ev| new_source_name.set(event_target_value(&ev))
                                />
                            </div>
                            <div>
                                <label class="block mb-1 text-xs text-gray-500 dark:text-gray-400">"Base URL"</label>
                                <input
                                    type="text"
                                    placeholder="https://example.com/designs"
                                    class="block w-full p-2 text-sm text-gray-900 bg-white rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
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
            }}

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
                let on_edit = on_edit.clone();
                view! {
                    <div class="space-y-8">
                        {groups.into_iter().map(|(source, ds)| {
                            let source_name = source.name.clone();
                            let source_url = source.base_url.clone();
                            let is_removable = source_url != DEFAULT_DESIGNS_URL && source_url != LOCAL_SOURCE;
                            let url_for_remove = source_url.clone();
                            let on_edit = on_edit.clone();
                            view! {
                                <div>
                                    <div class="flex items-center justify-between mb-3">
                                        <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
                                            {source_name}
                                            {if source_url == DEFAULT_DESIGNS_URL {
                                                view! {
                                                    <a
                                                        href="https://github.com/elsirion/paper-ecash-note-designs"
                                                        target="_blank"
                                                        rel="noopener noreferrer"
                                                        class="ml-1 text-xs font-normal text-gray-400 dark:text-gray-500 hover:text-blue-500 dark:hover:text-blue-400 transition-colors"
                                                    >
                                                        "(GitHub)"
                                                    </a>
                                                }.into_any()
                                            } else {
                                                view! { <span></span> }.into_any()
                                            }}
                                        </h3>
                                        {if is_removable {
                                            view! {
                                                <button
                                                    class="px-2 py-1 text-xs font-medium text-red-700 dark:text-red-400 border border-red-300 dark:border-red-800 hover:bg-red-50 dark:hover:bg-red-900/20 rounded transition-colors"
                                                    on:click=move |_| remove_source(url_for_remove.clone())
                                                >
                                                    "Remove"
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
                                        let on_edit = on_edit.clone();
                                        view! {
                                            <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4">
                                                {ds.into_iter().map(|d| {
                                                    let name = d.name.clone();
                                                    let front_url = d.front_url.clone();
                                                    let back_url = d.back_url.clone();
                                                    let pc = d.paper_color.clone().unwrap_or_else(|| "#ffffff".into());
                                                    let is_local = source_url == LOCAL_SOURCE;
                                                    let delete_id = d.id.clone();
                                                    let design_for_edit = d.clone();
                                                    let design_for_preview = d.clone();
                                                    let on_edit = on_edit.clone();
                                                    view! {
                                                        <div class="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
                                                            <div
                                                                class="flex gap-1 p-2"
                                                                style=format!("background-color: {pc};")
                                                            >
                                                                <img src=front_url class="w-1/2 h-auto rounded" style="mix-blend-mode: multiply;" />
                                                                <img src=back_url class="w-1/2 h-auto rounded" style="mix-blend-mode: multiply;" />
                                                            </div>
                                                            <div class="flex items-center justify-between p-3">
                                                                <span class="text-sm font-medium text-gray-900 dark:text-white">{name}</span>
                                                                <div class="flex gap-2">
                                                                    <button
                                                                        class="px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-lg transition-colors"
                                                                        on:click=move |_| {
                                                                            if let Some(at) = &design_for_preview.amount_text {
                                                                                fonts::inject_font_link_weighted(&at.font_family, at.font_weight);
                                                                            }
                                                                            preview_design.set(Some(design_for_preview.clone()));
                                                                        }
                                                                    >
                                                                        "Preview"
                                                                    </button>
                                                                    <button
                                                                        class="px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-lg transition-colors"
                                                                        on:click=move |_| on_edit(design_for_edit.clone())
                                                                    >
                                                                        "Fork"
                                                                    </button>
                                                                    {if is_local {
                                                                        let delete_id = delete_id.clone();
                                                                        view! {
                                                                            <button
                                                                                class="p-1.5 text-gray-400 dark:text-gray-500 hover:text-red-600 dark:hover:text-red-400 transition-colors"
                                                                                title="Delete local design"
                                                                                on:click=move |_| {
                                                                                    storage::delete_local_design(&delete_id);
                                                                                    // Refresh local group
                                                                                    let mut groups = source_groups.get_untracked();
                                                                                    for (src, ds) in groups.iter_mut() {
                                                                                        if src.base_url == LOCAL_SOURCE {
                                                                                            ds.retain(|d| d.id != delete_id);
                                                                                        }
                                                                                    }
                                                                                    groups.retain(|(src, ds)| src.base_url != LOCAL_SOURCE || !ds.is_empty());
                                                                                    source_groups.set(groups);
                                                                                }
                                                                            >
                                                                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                                                                                </svg>
                                                                            </button>
                                                                        }.into_any()
                                                                    } else {
                                                                        view! { <span></span> }.into_any()
                                                                    }}
                                                                </div>
                                                            </div>
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

            // Preview panel
            {move || {
                let Some(design) = preview_design.get() else {
                    return view! { <div></div> }.into_any();
                };
                let name = design.name.clone();
                let front = design.front_url.clone();
                let back = design.back_url.clone();
                let qr_left = design.qr_x_offset_cm / NOTE_WIDTH_CM * 100.0;
                let qr_top = design.qr_y_offset_cm / NOTE_HEIGHT_CM * 100.0;
                let qr_w = design.qr_size_cm / NOTE_WIDTH_CM * 100.0;
                let qr_h = design.qr_size_cm / NOTE_HEIGHT_CM * 100.0;
                let sample_qr = make_sample_qr_url(design.qr_error_correction);
                let overlay = design.qr_overlay_url.clone();
                let amount_text = design.amount_text.clone();
                let has_amount_text = amount_text.is_some();
                paper_color.set(design.paper_color.clone().unwrap_or_else(|| "#ffffff".into()));
                view! {
                    <div class="mt-8 p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <div class="flex items-center justify-between mb-4">
                            <h3 class="text-sm font-semibold text-gray-900 dark:text-white">
                                "Preview: " {name}
                            </h3>
                            <button
                                class="px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                                on:click=move |_| preview_design.set(None)
                            >
                                "Close"
                            </button>
                        </div>

                        <div class="flex flex-wrap items-end gap-4 mb-4">
                            {if has_amount_text {
                                view! {
                                    <div>
                                        <label class="block mb-1 text-xs text-gray-500 dark:text-gray-400">"Amount Text"</label>
                                        <input
                                            type="text"
                                            class="block w-full max-w-xs p-2 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                            prop:value=move || preview_text.get()
                                            on:input=move |ev| preview_text.set(event_target_value(&ev))
                                        />
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                            <div>
                                <label class="block mb-1 text-xs text-gray-500 dark:text-gray-400">"Paper Color"</label>
                                <input
                                    type="color"
                                    class="h-[38px] w-16 p-1 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 cursor-pointer"
                                    prop:value=move || paper_color.get()
                                    on:input=move |ev| paper_color.set(event_target_value(&ev))
                                />
                            </div>
                        </div>

                        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                            // Front preview with QR + amount text
                            <div>
                                <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">"Front"</p>
                                <div class="border border-gray-300 dark:border-gray-600 rounded-lg overflow-hidden">
                                    <div
                                        class="relative w-full"
                                        style=move || format!(
                                            "aspect-ratio: 2 / 1; container-type: size; background-color: {};",
                                            paper_color.get()
                                        )
                                    >
                                        <img
                                            src=front
                                            class="absolute inset-0 w-full h-full object-fill pointer-events-none"
                                            style="mix-blend-mode: multiply;"
                                            draggable="false"
                                        />
                                        <div
                                            class="absolute pointer-events-none"
                                            style=format!(
                                                "left: {qr_left:.2}%; top: {qr_top:.2}%; width: {qr_w:.2}%; height: {qr_h:.2}%;",
                                            )
                                        >
                                            <img
                                                src=sample_qr
                                                class="w-full h-full object-fill"
                                                style="mix-blend-mode: multiply;"
                                                draggable="false"
                                            />
                                            {overlay.map(|url| view! {
                                                <img
                                                    src=url
                                                    class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[20%] h-[20%] object-contain"
                                                    draggable="false"
                                                />
                                            })}
                                        </div>
                                        {amount_text.as_ref().map(|at| {
                                            let font = at.font_family.clone();
                                            let weight = at.font_weight;
                                            let x_pct = at.x_offset_cm / NOTE_WIDTH_CM * 100.0;
                                            let y_pct = at.y_offset_cm / NOTE_HEIGHT_CM * 100.0;
                                            let w_pct = at.width_cm / NOTE_WIDTH_CM * 100.0;
                                            let h_pct = at.height_cm / NOTE_HEIGHT_CM * 100.0;
                                            let fs = h_pct * 0.75;
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
                                                            "font-family: '{font}', sans-serif; font-size: {fs:.3}cqh; font-weight: {weight}; color: black; line-height: 1;",
                                                        )
                                                    >
                                                        {move || preview_text.get()}
                                                    </span>
                                                </div>
                                            }
                                        })}
                                    </div>
                                </div>
                            </div>
                            // Back preview
                            <div>
                                <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">"Back"</p>
                                <div class="border border-gray-300 dark:border-gray-600 rounded-lg overflow-hidden">
                                    <div
                                        class="relative w-full"
                                        style=move || format!(
                                            "aspect-ratio: 2 / 1; background-color: {};",
                                            paper_color.get()
                                        )
                                    >
                                        <img
                                            src=back
                                            class="absolute inset-0 w-full h-full object-fill pointer-events-none"
                                            style="mix-blend-mode: multiply;"
                                            draggable="false"
                                        />
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                }.into_any()
            }}
        </div>
    }
}
