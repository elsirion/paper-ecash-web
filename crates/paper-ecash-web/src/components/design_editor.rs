use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::browser;
use crate::fonts;
use crate::models::QrErrorCorrection;
use crate::qr;

const NOTE_WIDTH_CM: f64 = 14.0;
const NOTE_HEIGHT_CM: f64 = 7.0;
const MIN_QR_SIZE_CM: f64 = 0.5;

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
            web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts)
                .ok()
                .and_then(|b| web_sys::Url::create_object_url_with_blob(&b).ok())
                .unwrap_or_default()
        }
        Err(_) => String::new(),
    }
}

fn clamp(v: f64, min: f64, max: f64) -> f64 {
    v.max(min).min(max)
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[derive(Clone, Copy, PartialEq)]
enum DragMode {
    Move,
    Resize,
    TextMove,
    TextResize,
}

#[component]
pub fn DesignEditor(on_back: impl Fn() + Send + Sync + 'static) -> impl IntoView {
    let design_id = RwSignal::new(String::new());
    let design_name = RwSignal::new(String::new());
    let id_manually_edited = RwSignal::new(false);
    let front_url = RwSignal::new(Option::<String>::None);
    let back_url = RwSignal::new(Option::<String>::None);
    let overlay_url = RwSignal::new(Option::<String>::None);
    let qr_x = RwSignal::new(0.0f64);
    let qr_y = RwSignal::new(0.0f64);
    let qr_size = RwSignal::new(7.0f64);
    let ec_level = RwSignal::new(String::from("M"));

    // Amount text signals (placement/style, NOT content)
    let text_enabled = RwSignal::new(false);
    let text_font = RwSignal::new(String::from("Roboto"));
    let text_font_search = RwSignal::new(String::new());
    let text_color = RwSignal::new(String::from("#000000"));
    let text_x = RwSignal::new(1.0f64);
    let text_y = RwSignal::new(0.5f64);
    let text_w = RwSignal::new(4.0f64);
    let text_h = RwSignal::new(1.0f64);
    let show_font_dropdown = RwSignal::new(false);
    let text_font_url = RwSignal::new(String::new());

    // Drag state
    let dragging = RwSignal::new(Option::<DragMode>::None);
    let drag_start_x = RwSignal::new(0.0f64);
    let drag_start_y = RwSignal::new(0.0f64);
    let drag_start_qr_x = RwSignal::new(0.0f64);
    let drag_start_qr_y = RwSignal::new(0.0f64);
    let drag_start_qr_size = RwSignal::new(0.0f64);
    let drag_start_text_x = RwSignal::new(0.0f64);
    let drag_start_text_y = RwSignal::new(0.0f64);
    let drag_start_text_w = RwSignal::new(0.0f64);
    let drag_start_text_h = RwSignal::new(0.0f64);
    let preview_ref = NodeRef::<leptos::html::Div>::new();

    let sample_qr_url = make_sample_qr_url();

    let on_front_change = read_file_to_signal(front_url);
    let on_back_change = read_file_to_signal(back_url);

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

    let select_font = move |family: String| {
        text_font.set(family.clone());
        text_font_search.set(family.clone());
        show_font_dropdown.set(false);
        fonts::inject_font_link(&family);
        wasm_bindgen_futures::spawn_local(async move {
            match fonts::fetch_font_woff2(&family).await {
                Ok((url, _)) => text_font_url.set(url),
                Err(e) => tracing::warn!("Failed to fetch font woff2: {e}"),
            }
        });
    };

    // Load the default font on mount so font_url is never empty.
    select_font(text_font.get_untracked());

    let qr_left_pct = move || qr_x.get() / NOTE_WIDTH_CM * 100.0;
    let qr_top_pct = move || qr_y.get() / NOTE_HEIGHT_CM * 100.0;
    let qr_w_pct = move || qr_size.get() / NOTE_WIDTH_CM * 100.0;
    let qr_h_pct = move || qr_size.get() / NOTE_HEIGHT_CM * 100.0;

    let on_qr_pointerdown = move |ev: web_sys::PointerEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        if let Some(target) = ev.target() {
            let el: web_sys::Element = target.unchecked_into();
            let _ = el.set_pointer_capture(ev.pointer_id());
        }
        dragging.set(Some(DragMode::Move));
        drag_start_x.set(ev.client_x() as f64);
        drag_start_y.set(ev.client_y() as f64);
        drag_start_qr_x.set(qr_x.get_untracked());
        drag_start_qr_y.set(qr_y.get_untracked());
    };

    let on_resize_pointerdown = move |ev: web_sys::PointerEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        if let Some(target) = ev.target() {
            let el: web_sys::Element = target.unchecked_into();
            let _ = el.set_pointer_capture(ev.pointer_id());
        }
        dragging.set(Some(DragMode::Resize));
        drag_start_x.set(ev.client_x() as f64);
        drag_start_y.set(ev.client_y() as f64);
        drag_start_qr_size.set(qr_size.get_untracked());
        drag_start_qr_x.set(qr_x.get_untracked());
        drag_start_qr_y.set(qr_y.get_untracked());
    };

    let on_text_pointerdown = move |ev: web_sys::PointerEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        if let Some(target) = ev.target() {
            let el: web_sys::Element = target.unchecked_into();
            let _ = el.set_pointer_capture(ev.pointer_id());
        }
        dragging.set(Some(DragMode::TextMove));
        drag_start_x.set(ev.client_x() as f64);
        drag_start_y.set(ev.client_y() as f64);
        drag_start_text_x.set(text_x.get_untracked());
        drag_start_text_y.set(text_y.get_untracked());
    };

    let on_text_resize_pointerdown = move |ev: web_sys::PointerEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        if let Some(target) = ev.target() {
            let el: web_sys::Element = target.unchecked_into();
            let _ = el.set_pointer_capture(ev.pointer_id());
        }
        dragging.set(Some(DragMode::TextResize));
        drag_start_x.set(ev.client_x() as f64);
        drag_start_y.set(ev.client_y() as f64);
        drag_start_text_w.set(text_w.get_untracked());
        drag_start_text_h.set(text_h.get_untracked());
        drag_start_text_x.set(text_x.get_untracked());
        drag_start_text_y.set(text_y.get_untracked());
    };

    let on_preview_pointermove = move |ev: web_sys::PointerEvent| {
        let Some(mode) = dragging.get_untracked() else { return };
        ev.prevent_default();
        let Some(el) = preview_ref.get() else { return };
        let el_ref: &web_sys::Element = el.as_ref();
        let rect = el_ref.get_bounding_client_rect();
        let container_w = rect.width();
        let container_h = rect.height();
        if container_w == 0.0 || container_h == 0.0 {
            return;
        }
        let dx_px = ev.client_x() as f64 - drag_start_x.get_untracked();
        let dy_px = ev.client_y() as f64 - drag_start_y.get_untracked();
        let dx_cm = dx_px / container_w * NOTE_WIDTH_CM;
        let dy_cm = dy_px / container_h * NOTE_HEIGHT_CM;
        match mode {
            DragMode::Move => {
                let start_x = drag_start_qr_x.get_untracked();
                let start_y = drag_start_qr_y.get_untracked();
                let size = qr_size.get_untracked();
                let new_x = clamp(start_x + dx_cm, 0.0, NOTE_WIDTH_CM - size);
                let new_y = clamp(start_y + dy_cm, 0.0, NOTE_HEIGHT_CM - size);
                qr_x.set(round2(new_x));
                qr_y.set(round2(new_y));
            }
            DragMode::Resize => {
                let delta_cm = if dx_cm.abs() > dy_cm.abs() { dx_cm } else { dy_cm };
                let start_size = drag_start_qr_size.get_untracked();
                let start_x = drag_start_qr_x.get_untracked();
                let start_y = drag_start_qr_y.get_untracked();
                let max_size = (NOTE_WIDTH_CM - start_x).min(NOTE_HEIGHT_CM - start_y);
                let new_size = clamp(start_size + delta_cm, MIN_QR_SIZE_CM, max_size);
                qr_size.set(round2(new_size));
            }
            DragMode::TextMove => {
                let start_x = drag_start_text_x.get_untracked();
                let start_y = drag_start_text_y.get_untracked();
                let tw = text_w.get_untracked();
                let th = text_h.get_untracked();
                let new_x = clamp(start_x + dx_cm, 0.0, NOTE_WIDTH_CM - tw);
                let new_y = clamp(start_y + dy_cm, 0.0, NOTE_HEIGHT_CM - th);
                text_x.set(round2(new_x));
                text_y.set(round2(new_y));
            }
            DragMode::TextResize => {
                let start_w = drag_start_text_w.get_untracked();
                let start_h = drag_start_text_h.get_untracked();
                let tx = drag_start_text_x.get_untracked();
                let ty = drag_start_text_y.get_untracked();
                let new_w = clamp(start_w + dx_cm, 0.5, NOTE_WIDTH_CM - tx);
                let new_h = clamp(start_h + dy_cm, 0.3, NOTE_HEIGHT_CM - ty);
                text_w.set(round2(new_w));
                text_h.set(round2(new_h));
            }
        }
    };

    let on_preview_pointerup = move |_ev: web_sys::PointerEvent| {
        dragging.set(None);
    };

    let can_download = move || {
        !design_id.get().trim().is_empty()
            && !design_name.get().trim().is_empty()
            && front_url.get().is_some()
            && back_url.get().is_some()
    };

    // Build the amount_text TextConfig (without .text — that's set at issuance time)
    let build_amount_text = move || -> Option<crate::models::TextConfig> {
        if !text_enabled.get_untracked() {
            return None;
        }
        let box_h = text_h.get_untracked();
        let box_w = text_w.get_untracked();
        // Font size is 75% of the box height (converted cm → pt)
        let font_size_pt = box_h * 0.75 * 28.3465;
        Some(crate::models::TextConfig {
            font_family: text_font.get_untracked(),
            font_url: text_font_url.get_untracked(),
            font_size_pt,
            color_hex: text_color.get_untracked(),
            x_offset_cm: text_x.get_untracked(),
            y_offset_cm: text_y.get_untracked(),
            width_cm: box_w,
            height_cm: box_h,
            text: None,
        })
    };

    let build_design_json = move || -> Option<String> {
        let id = design_id.get_untracked().trim().to_string();
        let name = design_name.get_untracked().trim().to_string();
        if id.is_empty() || name.is_empty() {
            return None;
        }
        let mut qr_obj = serde_json::json!({
            "x_offset_cm": qr_x.get_untracked(),
            "y_offset_cm": qr_y.get_untracked(),
            "size_cm": qr_size.get_untracked(),
            "error_correction": ec_level.get_untracked(),
        });
        if overlay_url.get_untracked().is_some() {
            qr_obj.as_object_mut().unwrap()
                .insert("overlay".into(), "qr_overlay.png".into());
        }
        let mut json = serde_json::json!({
            "id": id,
            "name": name,
            "front": "front.png",
            "back": "back.png",
            "qr": qr_obj,
        });
        if text_enabled.get_untracked() {
            let box_h = text_h.get_untracked();
            json.as_object_mut().unwrap().insert("amount_text".into(), serde_json::json!({
                "font_family": text_font.get_untracked(),
                "font_url": text_font_url.get_untracked(),
                "font_size_pt": box_h * 0.75 * 28.3465,
                "color_hex": text_color.get_untracked(),
                "x_offset_cm": text_x.get_untracked(),
                "y_offset_cm": text_y.get_untracked(),
                "width_cm": text_w.get_untracked(),
                "height_cm": box_h,
            }));
        }
        Some(serde_json::to_string_pretty(&json).unwrap_or_default())
    };

    let download_json = move |_| {
        if let Some(json_str) = build_design_json() {
            browser::download_file(json_str.as_bytes(), "design.json", "application/json");
        }
    };

    let build_local_design = move || -> Option<crate::designs::Design> {
        let id = design_id.get_untracked().trim().to_string();
        let name = design_name.get_untracked().trim().to_string();
        let front = front_url.get_untracked()?;
        let back = back_url.get_untracked()?;
        if id.is_empty() || name.is_empty() {
            return None;
        }
        Some(crate::designs::Design {
            id,
            name,
            front_url: front,
            back_url: back,
            qr_x_offset_cm: qr_x.get_untracked(),
            qr_y_offset_cm: qr_y.get_untracked(),
            qr_size_cm: qr_size.get_untracked(),
            qr_error_correction: match ec_level.get_untracked().as_str() {
                "Q" => QrErrorCorrection::Q,
                "H" => QrErrorCorrection::H,
                _ => QrErrorCorrection::M,
            },
            qr_overlay_url: overlay_url.get_untracked(),
            amount_text: build_amount_text(),
        })
    };

    let on_back = std::sync::Arc::new(on_back);
    let on_back_use = on_back.clone();
    let on_back_btn = on_back.clone();

    let save_and_use = move |_| {
        if let Some(design) = build_local_design() {
            crate::storage::save_local_design(&design);
            on_back_use();
        }
    };

    let can_save = move || {
        !design_id.get().trim().is_empty()
            && !design_name.get().trim().is_empty()
            && front_url.get().is_some()
            && back_url.get().is_some()
            && (!text_enabled.get() || !text_font_url.get().is_empty())
    };

    let qr_cursor = move || {
        match dragging.get() {
            Some(DragMode::Move) | Some(DragMode::TextMove) => "grabbing",
            Some(DragMode::Resize) | Some(DragMode::TextResize) => "nwse-resize",
            None => "grab",
        }
    };

    view! {
        <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 sm:p-6">
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">"Design Editor"</h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-6">
                "Create a note design template with live QR code placement preview."
            </p>

            // Name and ID
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-6">
                <div>
                    <label class="block mb-1 text-sm font-medium text-gray-900 dark:text-white">"Design Name"</label>
                    <input
                        type="text"
                        placeholder="My Design"
                        class="block w-full p-2.5 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white"
                        prop:value=move || design_name.get()
                        on:input=move |ev| {
                            let name = event_target_value(&ev);
                            design_name.set(name.clone());
                            if !id_manually_edited.get_untracked() {
                                let id = name.trim().to_lowercase()
                                    .replace(|c: char| !c.is_alphanumeric(), "_")
                                    .trim_matches('_').to_string();
                                design_id.set(id);
                            }
                        }
                    />
                </div>
                <div>
                    <label class="block mb-1 text-sm font-medium text-gray-900 dark:text-white">"Design ID"</label>
                    <input
                        type="text"
                        placeholder="my_design"
                        class="block w-full p-2.5 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white"
                        prop:value=move || design_id.get()
                        on:input=move |ev| {
                            id_manually_edited.set(true);
                            design_id.set(event_target_value(&ev));
                        }
                    />
                    <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">"Auto-generated from name. Edit to override."</p>
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
                            step="any"
                            min="0"
                            class="block w-full p-2 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            prop:value=move || format!("{:.2}", qr_x.get())
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
                            step="any"
                            min="0"
                            class="block w-full p-2 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            prop:value=move || format!("{:.2}", qr_y.get())
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
                            step="any"
                            min="0.5"
                            class="block w-full p-2 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            prop:value=move || format!("{:.2}", qr_size.get())
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
                <p class="mt-2 text-xs text-gray-500 dark:text-gray-400">
                    "Drag the QR code in the preview to reposition. Drag the corner handle to resize."
                </p>
            </div>

            // Amount Text Settings
            <div class="mb-6">
                <div class="flex items-center gap-2 mb-3">
                    <input
                        type="checkbox"
                        class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 dark:bg-gray-700 dark:border-gray-600"
                        prop:checked=move || text_enabled.get()
                        on:change=move |ev| text_enabled.set(event_target_checked(&ev))
                    />
                    <h3 class="text-sm font-semibold text-gray-900 dark:text-white">"Amount Text Placement"</h3>
                </div>
                <p class="text-xs text-gray-500 dark:text-gray-400 mb-3">
                    "Define where amount text will appear. The actual text is entered when selecting this design."
                </p>

                {move || {
                    if !text_enabled.get() {
                        return view! { <div></div> }.into_any();
                    }
                    let select_font_clone = select_font.clone();
                    view! {
                        <div class="grid grid-cols-2 sm:grid-cols-4 gap-4">
                            // Font picker
                            <div class="col-span-3 relative">
                                <label class="block mb-1 text-xs text-gray-500 dark:text-gray-400">"Font"</label>
                                <input
                                    type="text"
                                    placeholder="Search fonts..."
                                    class="block w-full p-2 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                    prop:value=move || text_font_search.get()
                                    on:input=move |ev| {
                                        text_font_search.set(event_target_value(&ev));
                                        show_font_dropdown.set(true);
                                    }
                                    on:focus=move |_| show_font_dropdown.set(true)
                                />
                                {move || {
                                    if !show_font_dropdown.get() {
                                        return view! { <div></div> }.into_any();
                                    }
                                    let search = text_font_search.get().to_lowercase();
                                    let has_search = !search.trim().is_empty();
                                    let entries: Vec<&str> = if has_search {
                                        fonts::GOOGLE_FONTS.iter().copied()
                                            .filter(|f| f.to_lowercase().contains(&search))
                                            .collect()
                                    } else {
                                        fonts::GOOGLE_FONTS.iter().copied().take(20).collect()
                                    };
                                    if entries.is_empty() {
                                        return view! {
                                            <div class="absolute z-10 mt-1 w-full bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg shadow-lg p-3 text-sm text-gray-500 dark:text-gray-400">
                                                "No fonts found"
                                            </div>
                                        }.into_any();
                                    }
                                    for family in &entries {
                                        fonts::inject_font_link(family);
                                    }
                                    let select_font_inner = select_font_clone.clone();
                                    let heading = if has_search { "Search results" } else { "Popular fonts" };
                                    view! {
                                        <div class="absolute z-10 mt-1 w-full max-h-64 overflow-y-auto bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg shadow-lg">
                                            <div class="px-3 py-1.5 text-xs text-gray-400 dark:text-gray-500 border-b border-gray-200 dark:border-gray-600">
                                                {heading}
                                            </div>
                                            {entries.into_iter().map(|family| {
                                                let family_owned = family.to_string();
                                                let family2 = family_owned.clone();
                                                let family3 = family_owned.clone();
                                                let select_fn = select_font_inner.clone();
                                                view! {
                                                    <button
                                                        class="block w-full text-left px-3 py-2 text-sm text-gray-900 dark:text-white hover:bg-gray-100 dark:hover:bg-gray-600"
                                                        on:click=move |_| select_fn(family_owned.clone())
                                                    >
                                                        <span class="text-xs text-gray-400 dark:text-gray-500">{family2.clone()}</span>
                                                        <span
                                                            class="block text-base"
                                                            style=format!("font-family: '{}', sans-serif;", family2)
                                                        >
                                                            {format!("The quick brown fox — {}", family3)}
                                                        </span>
                                                    </button>
                                                }
                                            }).collect_view()}
                                        </div>
                                    }.into_any()
                                }}
                            </div>

                            // Color
                            <div>
                                <label class="block mb-1 text-xs text-gray-500 dark:text-gray-400">"Color"</label>
                                <input
                                    type="color"
                                    class="block w-full h-[38px] p-1 text-sm bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 cursor-pointer"
                                    prop:value=move || text_color.get()
                                    on:input=move |ev| text_color.set(event_target_value(&ev))
                                />
                            </div>
                        </div>
                    }.into_any()
                }}
            </div>

            // Live Preview
            <div class="mb-6">
                <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-3">"Live Preview"</h3>
                {move || {
                    if let Some(front) = front_url.get() {
                        let sample_qr = sample_qr_url.clone();
                        view! {
                            <div class="border border-gray-300 dark:border-gray-600 rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-900">
                                <div
                                    node_ref=preview_ref
                                    class="relative w-full select-none"
                                    style="aspect-ratio: 2 / 1; touch-action: none; container-type: size;"
                                    on:pointermove=on_preview_pointermove
                                    on:pointerup=on_preview_pointerup
                                    on:pointercancel=move |_ev: web_sys::PointerEvent| dragging.set(None)
                                >
                                    <img
                                        src=front
                                        class="absolute inset-0 w-full h-full object-fill pointer-events-none"
                                        draggable="false"
                                    />
                                    // QR code overlay
                                    <div
                                        class="absolute border-2 border-dashed border-blue-500 dark:border-blue-400 overflow-visible"
                                        style=move || format!(
                                            "left: {:.2}%; top: {:.2}%; width: {:.2}%; height: {:.2}%; cursor: {};",
                                            qr_left_pct(), qr_top_pct(), qr_w_pct(), qr_h_pct(), qr_cursor()
                                        )
                                        on:pointerdown=on_qr_pointerdown
                                    >
                                        <img
                                            src=sample_qr.clone()
                                            class="w-full h-full object-fill qr-image pointer-events-none"
                                            draggable="false"
                                        />
                                        {move || overlay_url.get().map(|url| view! {
                                            <img
                                                src=url
                                                class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[20%] h-[20%] object-contain pointer-events-none"
                                                draggable="false"
                                            />
                                        })}
                                        <div
                                            class="absolute -bottom-2 -right-2 w-5 h-5 bg-blue-500 dark:bg-blue-400 rounded-sm border-2 border-white dark:border-gray-900 shadow-md"
                                            style="cursor: nwse-resize; touch-action: none;"
                                            on:pointerdown=on_resize_pointerdown
                                        ></div>
                                    </div>
                                    // Amount text box
                                    {move || {
                                        if !text_enabled.get() {
                                            return view! { <span></span> }.into_any();
                                        }
                                        let font = text_font.get();
                                        let color = text_color.get();
                                        let x_pct = text_x.get() / NOTE_WIDTH_CM * 100.0;
                                        let y_pct = text_y.get() / NOTE_HEIGHT_CM * 100.0;
                                        let w_pct = text_w.get() / NOTE_WIDTH_CM * 100.0;
                                        let h_pct = text_h.get() / NOTE_HEIGHT_CM * 100.0;
                                        view! {
                                            <div
                                                class="absolute border-2 border-dashed border-blue-500 dark:border-blue-400 overflow-hidden select-none flex items-center justify-center"
                                                style=move || format!(
                                                    "left: {x_pct:.2}%; top: {y_pct:.2}%; width: {w_pct:.2}%; height: {h_pct:.2}%; cursor: grab; touch-action: none;",
                                                )
                                                on:pointerdown=on_text_pointerdown
                                            >
                                                <span
                                                    class="pointer-events-none whitespace-nowrap"
                                                    style=move || {
                                                        let fs = h_pct * 0.75;
                                                        format!(
                                                            "font-family: '{font}', sans-serif; font-size: {fs:.3}cqh; color: {color}; line-height: 1;",
                                                        )
                                                    }
                                                >
                                                    "1000 sats"
                                                </span>
                                                <div
                                                    class="absolute -bottom-2 -right-2 w-5 h-5 bg-blue-500 dark:bg-blue-400 rounded-sm border-2 border-white dark:border-gray-900 shadow-md"
                                                    style="cursor: nwse-resize; touch-action: none;"
                                                    on:pointerdown=on_text_resize_pointerdown
                                                ></div>
                                            </div>
                                        }.into_any()
                                    }}
                                </div>
                                <div class="px-3 py-1.5 text-xs text-gray-500 dark:text-gray-400 text-center bg-gray-50 dark:bg-gray-800 border-t border-gray-300 dark:border-gray-600">
                                    "Front side preview (14cm \u{00d7} 7cm note area)"
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
            </div>

            // Actions
            <div class="flex flex-col-reverse sm:flex-row gap-3 sm:justify-end">
                <button
                    class="px-5 py-2.5 text-sm font-medium text-gray-900 bg-white border border-gray-300 rounded-lg hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:focus:ring-gray-700 transition-colors"
                    on:click=move |_| on_back_btn()
                >
                    "Back"
                </button>
                <button
                    class="px-5 py-2.5 text-sm font-medium text-gray-900 bg-white border border-gray-300 rounded-lg hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:focus:ring-gray-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    disabled=move || !can_download()
                    on:click=download_json
                >
                    "Download design.json"
                </button>
                <button
                    class="px-5 py-2.5 text-sm font-medium text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    disabled=move || !can_save()
                    on:click=save_and_use
                >
                    "Save & Use"
                </button>
            </div>
        </div>
    }
}
