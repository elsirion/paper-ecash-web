use js_sys::Promise;
use wasm_bindgen::prelude::*;
use web_sys::{FileSystemSyncAccessHandle, Worker};

#[wasm_bindgen(module = "/src/browser.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = openWalletDb)]
    pub fn open_wallet_db(file_name: &str) -> Result<Promise, JsValue>;

    #[wasm_bindgen(catch, js_name = copyText)]
    pub fn copy_text(value: &str) -> Result<Promise, JsValue>;

    #[wasm_bindgen(catch, js_name = createWalletWorker)]
    pub fn create_wallet_worker() -> Result<Promise, JsValue>;

    #[wasm_bindgen(js_name = supportsSyncAccessHandles)]
    pub fn supports_sync_access_handles_js() -> bool;

    #[wasm_bindgen(js_name = downloadBlob)]
    pub fn download_blob_js(bytes: &[u8], filename: &str, mime_type: &str);

    #[wasm_bindgen(catch, js_name = fetchDesignImage)]
    pub fn fetch_design_image(url: &str) -> Result<Promise, JsValue>;

    #[wasm_bindgen(catch, js_name = fetchFontWoff2)]
    pub fn fetch_font_woff2_js(family: &str, weight: u16) -> Result<Promise, JsValue>;

    #[wasm_bindgen(catch, js_name = fetchJson)]
    pub fn fetch_json_js(url: &str) -> Result<Promise, JsValue>;
}

pub async fn open_wallet_handle(file_name: &str) -> anyhow::Result<FileSystemSyncAccessHandle> {
    let promise = open_wallet_db(file_name).map_err(js_error)?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(js_error)?;
    Ok(value.unchecked_into())
}

pub async fn spawn_wallet_worker() -> anyhow::Result<Worker> {
    let promise = create_wallet_worker().map_err(js_error)?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(js_error)?;
    Ok(value.unchecked_into())
}

pub fn supports_sync_access_handles() -> bool {
    supports_sync_access_handles_js()
}

pub fn is_worker_context() -> bool {
    web_sys::window().is_none()
}

pub async fn copy_to_clipboard(value: &str) -> anyhow::Result<()> {
    let promise = copy_text(value).map_err(js_error)?;
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(js_error)?;
    Ok(())
}

pub fn download_file(bytes: &[u8], filename: &str, mime_type: &str) {
    download_blob_js(bytes, filename, mime_type);
}

/// Create an object URL for the given PNG bytes.
pub fn png_object_url(png: &[u8]) -> String {
    let array = js_sys::Uint8Array::from(png);
    let parts = js_sys::Array::new();
    parts.push(&array.buffer());
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("image/png");
    web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts)
        .ok()
        .and_then(|b| web_sys::Url::create_object_url_with_blob(&b).ok())
        .unwrap_or_default()
}

pub async fn fetch_json(url: &str) -> anyhow::Result<String> {
    let promise = fetch_json_js(url).map_err(js_error)?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(js_error)?;
    value
        .as_string()
        .ok_or_else(|| anyhow::anyhow!("Expected string from fetchJson"))
}

pub async fn fetch_image_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
    let promise = fetch_design_image(url).map_err(js_error)?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(js_error)?;
    let array = js_sys::Uint8Array::new(&value);
    Ok(array.to_vec())
}

pub fn js_error(err: JsValue) -> anyhow::Error {
    anyhow::anyhow!(err.as_string().unwrap_or_else(|| format!("{err:?}")))
}
