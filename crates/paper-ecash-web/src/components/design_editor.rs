use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::browser;
use crate::models::QrErrorCorrection;
use crate::qr;

const NOTE_WIDTH_CM: f64 = 14.0;
const NOTE_HEIGHT_CM: f64 = 7.0;

fn read_file_to_signal(signal: RwSignal<Option<String>>) -> impl Fn(web_sys::Event) + 'static {
    move |ev: web_sys::Event| {
        let Some(target) = ev.target() else { return };
        let input: web_sys::HtmlInputElement = target.unchecked_into();
        let Some(files) = input.files() else { return };
        let file: web_sys::File = match files.get(0) {
            Some(f) => f,
            None => return,
        };
        let Ok(reader) = web_sys::FileReader::new() else { return };
        let reader2 = reader.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::ProgressEvent| {
            if let Ok(result) = reader2.result() {
                signal.set(result.as_string());
            }
        }) as Box<dyn FnMut(_)>);
        reader.set_onload(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
        let _ = reader.read_as_data_url(&file);
    }
}

fn make_sample_qr_url() -> String {
    let sample = "fed11qgqzc2nhwden5te0vejkg6tdd9h8gepwvejkg6tdd9h8garhduhx6ct5d9hxgmmjv9kx7pqdsample";
    match qr::generate_qr_png_white(sample, QrErrorCorrection::M, 6) {
        Ok(png) => {
            let array = js_sys::Uint8Array::from(&png[..]);
            let parts = js_sys::Array::new();
            parts.push(&array.buffer());
            let opts = web_sys::BlobPropertyBag::new();
            opts.set_type("image/png");
            web_sys::Blob::new_with_u8_array_sequence_and_options(
                &parts,
                &opts,
            )
            .ok()
            .and_then(|b| web_sys::Url::create_object_url_with_blob(&b).ok())
            .unwrap_or_default()
        }
        Err(_) => String::new(),
    }
}

#[component]
pub fn DesignEditor(on_back: impl Fn() + Send + Sync + 'static) -> impl IntoView {
    let design_id = RwSignal::new(String::new());
    let design_name = RwSignal::new(String::new());
    let front_url = RwSignal::new(Option::<String>::None);
    let back_url = RwSignal::new(Option::<String>::None);
    let overlay_url = RwSignal::new(Option::<String>::None);
    let qr_x = RwSignal::new(0.0f64);
    let qr_y = RwSignal::new(0.0f64);
    let qr_size = RwSignal::new(7.0f64);
    let ec_level = RwSignal::new(String::from("M"));

    let sample_qr_url = make_sample_qr_url();

    let on_front_change = read_file_to_signal(front_url);
    let on_back_change = read_file_to_signal(back_url);

    // Overlay change handler with auto EC upgrade
    let on_overlay_change = move |ev: web_sys::Event| {
        let Some(target) = ev.target() else { return };
        let input: web_sys::HtmlInputElement = target.unchecked_into();
        let Some(files) = input.files() else { return };
        let file: web_sys::File = match files.get(0) {
            Some(f) => f,
            None => return,
        };
        let Ok(reader) = web_sys::FileReader::new() else { return };
        let reader2 = reader.clone();
        let cb = Closure::wrap(Box::new(move |_: web_sys::ProgressEvent| {
            if let Ok(result) = reader2.result() {
                overlay_url.set(result.as_string());
                // Auto-upgrade EC when overlay is added
                if ec_level.get_untracked() == "M" {
                    ec_level.set("Q".into());
                }
            }
        }) as Box<dyn FnMut(_)>);
        reader.set_onload(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
        let _ = reader.read_as_data_url(&file);
    };

    let remove_overlay = move |_| {
        overlay_url.set(None);
    };

    // QR position as percentage of note area
    let qr_left_pct = move || qr_x.get() / NOTE_WIDTH_CM * 100.0;
    let qr_top_pct = move || qr_y.get() / NOTE_HEIGHT_CM * 100.0;
    let qr_w_pct = move || qr_size.get() / NOTE_WIDTH_CM * 100.0;
    let qr_h_pct = move || qr_size.get() / NOTE_HEIGHT_CM * 100.0;

    let can_download = move || {
        !design_id.get().trim().is_empty()
            && !design_name.get().trim().is_empty()
            && front_url.get().is_some()
            && back_url.get().is_some()
    };

    let download_json = move |_| {
        let id = design_id.get_untracked().trim().to_string();
        let name = design_name.get_untracked().trim().to_string();
        if id.is_empty() || name.is_empty() {
            return;
        }

        let mut qr_obj = serde_json::json!({
            "x_offset_cm": qr_x.get_untracked(),
            "y_offset_cm": qr_y.get_untracked(),
            "size_cm": qr_size.get_untracked(),
            "error_correction": ec_level.get_untracked(),
        });

        if overlay_url.get_untracked().is_some() {
            qr_obj
                .as_object_mut()
                .unwrap()
                .insert("overlay".into(), "qr_overlay.png".into());
        }

        let json = serde_json::json!({
            "id": id,
            "name": name,
            "front": "front.png",
            "back": "back.png",
            "qr": qr_obj,
        });

        let json_str = serde_json::to_string_pretty(&json).unwrap_or_default();
        browser::download_file(json_str.as_bytes(), "design.json", "application/json");
    };

    view! {
        <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 sm:p-6">
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">"Design Editor"</h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-6">
                "Create a note design template with live QR code placement preview."
            </p>

            // ID and Name
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-6">
                <div>
                    <label class="block mb-1 text-sm font-medium text-gray-900 dark:text-white">"Design ID"</label>
                    <input
                        type="text"
                        placeholder="my_design"
                        class="block w-full p-2.5 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white"
                        prop:value=move || design_id.get()
                        on:input=move |ev| design_id.set(event_target_value(&ev))
                    />
                    <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">"Lowercase, no spaces (used as folder name)"</p>
                </div>
                <div>
                    <label class="block mb-1 text-sm font-medium text-gray-900 dark:text-white">"Design Name"</label>
                    <input
                        type="text"
                        placeholder="My Design"
                        class="block w-full p-2.5 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white"
                        prop:value=move || design_name.get()
                        on:input=move |ev| design_name.set(event_target_value(&ev))
                    />
                </div>
            </div>

            // Image uploads
            <div class="space-y-4 mb-6">
                <div>
                    <label class="block mb-1 text-sm font-medium text-gray-900 dark:text-white">"Front Image"</label>
                    <div class="flex items-center gap-3">
                        <input
                            type="file"
                            accept="image/png,image/jpeg"
                            class="block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-medium file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100 dark:text-gray-400 dark:file:bg-blue-900/30 dark:file:text-blue-400"
                            on:change=on_front_change
                        />
                        {move || front_url.get().map(|url| view! {
                            <img src=url class="h-10 rounded border border-gray-300 dark:border-gray-600" />
                        })}
                    </div>
                </div>

                <div>
                    <label class="block mb-1 text-sm font-medium text-gray-900 dark:text-white">"Back Image"</label>
                    <div class="flex items-center gap-3">
                        <input
                            type="file"
                            accept="image/png,image/jpeg"
                            class="block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-medium file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100 dark:text-gray-400 dark:file:bg-blue-900/30 dark:file:text-blue-400"
                            on:change=on_back_change
                        />
                        {move || back_url.get().map(|url| view! {
                            <img src=url class="h-10 rounded border border-gray-300 dark:border-gray-600" />
                        })}
                    </div>
                </div>

                <div>
                    <label class="block mb-1 text-sm font-medium text-gray-900 dark:text-white">"QR Overlay (optional)"</label>
                    <div class="flex items-center gap-3">
                        {move || {
                            if overlay_url.get().is_some() {
                                view! {
                                    <div class="flex items-center gap-3 w-full">
                                        <span class="text-sm text-green-600 dark:text-green-400">"Overlay loaded"</span>
                                        <button
                                            class="px-3 py-1.5 text-xs font-medium text-red-700 dark:text-red-400 border border-red-300 dark:border-red-800 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors"
                                            on:click=remove_overlay
                                        >
                                            "Remove"
                                        </button>
                                        {overlay_url.get().map(|url| view! {
                                            <img src=url class="h-10 rounded border border-gray-300 dark:border-gray-600 bg-white" />
                                        })}
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <input
                                        type="file"
                                        accept="image/png"
                                        class="block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-medium file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100 dark:text-gray-400 dark:file:bg-blue-900/30 dark:file:text-blue-400"
                                        on:change=on_overlay_change.clone()
                                    />
                                }.into_any()
                            }
                        }}
                    </div>
                    <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                        "An icon placed in the center of the QR code. Automatically increases minimum error correction to Q."
                    </p>
                </div>
            </div>

            // QR Settings
            <div class="mb-6">
                <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-3">"QR Code Position & Size"</h3>
                <div class="grid grid-cols-2 sm:grid-cols-4 gap-4">
                    <div>
                        <label class="block mb-1 text-xs text-gray-500 dark:text-gray-400">"X Offset (cm)"</label>
                        <input
                            type="number"
                            step="0.05"
                            min="0"
                            class="block w-full p-2 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            prop:value=move || qr_x.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse() {
                                    qr_x.set(v);
                                }
                            }
                        />
                    </div>
                    <div>
                        <label class="block mb-1 text-xs text-gray-500 dark:text-gray-400">"Y Offset (cm)"</label>
                        <input
                            type="number"
                            step="0.05"
                            min="0"
                            class="block w-full p-2 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            prop:value=move || qr_y.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse() {
                                    qr_y.set(v);
                                }
                            }
                        />
                    </div>
                    <div>
                        <label class="block mb-1 text-xs text-gray-500 dark:text-gray-400">"Size (cm)"</label>
                        <input
                            type="number"
                            step="0.1"
                            min="0.5"
                            class="block w-full p-2 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            prop:value=move || qr_size.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    if v > 0.0 {
                                        qr_size.set(v);
                                    }
                                }
                            }
                        />
                    </div>
                    <div>
                        <label class="block mb-1 text-xs text-gray-500 dark:text-gray-400">"Error Correction"</label>
                        <select
                            class="block w-full p-2 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                // Enforce minimum Q when overlay exists
                                if overlay_url.get_untracked().is_some() && val == "M" {
                                    ec_level.set("Q".into());
                                } else {
                                    ec_level.set(val);
                                }
                            }
                            prop:value=move || ec_level.get()
                        >
                            <option value="M" disabled=move || overlay_url.get().is_some()>"M (15%)"</option>
                            <option value="Q">"Q (25%)"</option>
                            <option value="H">"H (30%)"</option>
                        </select>
                    </div>
                </div>
            </div>

            // Live Preview
            <div class="mb-6">
                <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-3">"Live Preview"</h3>
                {move || {
                    if let Some(front) = front_url.get() {
                        let sample_qr = sample_qr_url.clone();
                        view! {
                            <div class="border border-gray-300 dark:border-gray-600 rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-900">
                                <div class="relative w-full" style="aspect-ratio: 2 / 1;">
                                    <img
                                        src=front
                                        class="absolute inset-0 w-full h-full object-fill"
                                    />
                                    // QR code overlay
                                    <div
                                        class="absolute border-2 border-dashed border-blue-500 dark:border-blue-400 overflow-hidden"
                                        style=move || format!(
                                            "left: {:.2}%; top: {:.2}%; width: {:.2}%; height: {:.2}%;",
                                            qr_left_pct(), qr_top_pct(), qr_w_pct(), qr_h_pct()
                                        )
                                    >
                                        <img
                                            src=sample_qr.clone()
                                            class="w-full h-full object-fill qr-image"
                                        />
                                        // Overlay icon centered on QR
                                        {move || overlay_url.get().map(|url| view! {
                                            <img
                                                src=url
                                                class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[20%] h-[20%] object-contain"
                                            />
                                        })}
                                    </div>
                                </div>
                                <div class="px-3 py-1.5 text-xs text-gray-500 dark:text-gray-400 text-center bg-gray-50 dark:bg-gray-800 border-t border-gray-300 dark:border-gray-600">
                                    "Front side preview (14cm x 7cm note area)"
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="border-2 border-dashed border-gray-300 dark:border-gray-600 rounded-lg p-8 text-center text-gray-400 dark:text-gray-500" style="aspect-ratio: 2 / 1;">
                                <div class="flex items-center justify-center h-full">
                                    <p>"Upload a front image to see the preview"</p>
                                </div>
                            </div>
                        }.into_any()
                    }
                }}

                // Back image preview (smaller)
                {move || back_url.get().map(|url| view! {
                    <div class="mt-3 border border-gray-300 dark:border-gray-600 rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-900">
                        <div class="w-full sm:w-1/2">
                            <img src=url class="w-full" style="aspect-ratio: 2 / 1; object-fit: fill;" />
                        </div>
                        <div class="px-3 py-1.5 text-xs text-gray-500 dark:text-gray-400 text-center bg-gray-50 dark:bg-gray-800 border-t border-gray-300 dark:border-gray-600">
                            "Back side"
                        </div>
                    </div>
                })}
            </div>

            // Generated JSON preview
            <div class="mb-6">
                <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-3">"design.json"</h3>
                <pre class="bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg p-3 text-xs text-gray-800 dark:text-gray-300 overflow-x-auto font-mono">
                    {move || {
                        let id = design_id.get();
                        let name = design_name.get();
                        let has_overlay = overlay_url.get().is_some();
                        let mut qr_obj = serde_json::json!({
                            "x_offset_cm": qr_x.get(),
                            "y_offset_cm": qr_y.get(),
                            "size_cm": qr_size.get(),
                            "error_correction": ec_level.get(),
                        });
                        if has_overlay {
                            qr_obj.as_object_mut().unwrap()
                                .insert("overlay".into(), "qr_overlay.png".into());
                        }
                        let json = serde_json::json!({
                            "id": if id.is_empty() { "my_design".to_string() } else { id },
                            "name": if name.is_empty() { "My Design".to_string() } else { name },
                            "front": "front.png",
                            "back": "back.png",
                            "qr": qr_obj,
                        });
                        serde_json::to_string_pretty(&json).unwrap_or_default()
                    }}
                </pre>
            </div>

            // Actions
            <div class="flex flex-col-reverse sm:flex-row gap-3 sm:justify-end">
                <button
                    class="px-5 py-2.5 text-sm font-medium text-gray-900 bg-white border border-gray-300 rounded-lg hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:focus:ring-gray-700 transition-colors"
                    on:click=move |_| on_back()
                >
                    "Back"
                </button>
                <button
                    class="px-5 py-2.5 text-sm font-medium text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    disabled=move || !can_download()
                    on:click=download_json
                >
                    "Download design.json"
                </button>
            </div>
        </div>
    }
}
