use serde::Deserialize;
use wasm_bindgen::JsCast;

use crate::browser;

#[derive(Clone, Debug, Deserialize)]
pub struct FontEntry {
    pub family: String,
    #[serde(default)]
    pub category: String,
}

#[derive(Deserialize)]
struct FontMetadata {
    #[serde(rename = "familyMetadataList")]
    family_metadata_list: Vec<FontEntry>,
}

pub async fn fetch_font_list() -> anyhow::Result<Vec<FontEntry>> {
    let url = "https://fonts.google.com/metadata/fonts";
    let window = web_sys::window().ok_or_else(|| anyhow::anyhow!("no window"))?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| anyhow::anyhow!("not a Response"))?;
    if !resp.ok() {
        anyhow::bail!("HTTP {}", resp.status());
    }
    let text = wasm_bindgen_futures::JsFuture::from(
        resp.text().map_err(|e| anyhow::anyhow!("{e:?}"))?,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let json_str = text
        .as_string()
        .ok_or_else(|| anyhow::anyhow!("not a string"))?;
    // Strip XSS prefix ")]}'\n"
    let json_str = json_str
        .strip_prefix(")]}'")
        .unwrap_or(&json_str)
        .trim_start();
    let metadata: FontMetadata = serde_json::from_str(json_str)?;
    Ok(metadata.family_metadata_list)
}

pub async fn fetch_font_woff2(family: &str) -> anyhow::Result<(String, Vec<u8>)> {
    let promise = browser::fetch_font_woff2_js(family).map_err(browser::js_error)?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(browser::js_error)?;
    let url = js_sys::Reflect::get(&value, &"url".into())
        .map_err(browser::js_error)?
        .as_string()
        .ok_or_else(|| anyhow::anyhow!("no url in result"))?;
    let bytes_val = js_sys::Reflect::get(&value, &"bytes".into())
        .map_err(browser::js_error)?;
    let array = js_sys::Uint8Array::new(&bytes_val);
    Ok((url, array.to_vec()))
}
