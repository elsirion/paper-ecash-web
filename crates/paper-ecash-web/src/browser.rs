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

pub async fn fetch_image_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
    let promise = fetch_design_image(url).map_err(js_error)?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(js_error)?;
    let array = js_sys::Uint8Array::new(&value);
    Ok(array.to_vec())
}

fn js_error(err: JsValue) -> anyhow::Error {
    anyhow::anyhow!(err.as_string().unwrap_or_else(|| format!("{err:?}")))
}
