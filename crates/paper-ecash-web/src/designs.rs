use serde::Deserialize;
use wasm_bindgen::JsCast;

use crate::models::{QrErrorCorrection, TextConfig};

pub const DEFAULT_DESIGNS_URL: &str =
    "https://raw.githubusercontent.com/elsirion/paper-ecash-note-designs/main";

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct DesignSource {
    pub name: String,
    pub base_url: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Design {
    pub id: String,
    pub name: String,
    pub front_url: String,
    pub back_url: String,
    pub qr_x_offset_cm: f64,
    pub qr_y_offset_cm: f64,
    pub qr_size_cm: f64,
    pub qr_error_correction: QrErrorCorrection,
    pub qr_overlay_url: Option<String>,
    pub amount_text: Option<TextConfig>,
}

#[derive(Deserialize)]
struct DesignJson {
    id: String,
    name: String,
    front: String,
    back: String,
    qr: QrJson,
    #[serde(default)]
    amount_text: Option<TextConfig>,
}

#[derive(Deserialize)]
struct QrJson {
    x_offset_cm: f64,
    y_offset_cm: f64,
    size_cm: f64,
    error_correction: String,
    #[serde(default)]
    overlay: Option<String>,
}

fn parse_ec(s: &str) -> QrErrorCorrection {
    match s {
        "Q" => QrErrorCorrection::Q,
        "H" => QrErrorCorrection::H,
        _ => QrErrorCorrection::M,
    }
}

pub async fn fetch_designs() -> anyhow::Result<Vec<Design>> {
    fetch_designs_from(DEFAULT_DESIGNS_URL).await
}

pub async fn fetch_designs_from(base_url: &str) -> anyhow::Result<Vec<Design>> {
    let base_url = base_url.trim_end_matches('/');
    let index_url = format!("{base_url}/index.json");
    let ids: Vec<String> = fetch_json(&index_url).await?;

    let mut designs = Vec::with_capacity(ids.len());
    for id in ids {
        let design_url = format!("{base_url}/{id}/design.json");
        match fetch_json::<DesignJson>(&design_url).await {
            Ok(dj) => {
                let base = format!("{base_url}/{id}");
                let namespaced_id = format!("{base_url}:{}", dj.id);
                designs.push(Design {
                    id: namespaced_id,
                    name: dj.name,
                    front_url: format!("{base}/{}", dj.front),
                    back_url: format!("{base}/{}", dj.back),
                    qr_x_offset_cm: dj.qr.x_offset_cm,
                    qr_y_offset_cm: dj.qr.y_offset_cm,
                    qr_size_cm: dj.qr.size_cm,
                    qr_error_correction: parse_ec(&dj.qr.error_correction),
                    qr_overlay_url: dj.qr.overlay.map(|o| format!("{base}/{o}")),
                    amount_text: dj.amount_text,
                });
            }
            Err(e) => {
                tracing::warn!("Failed to load design {id}: {e}");
            }
        }
    }
    Ok(designs)
}

pub fn get_design(designs: &[Design], id: &str) -> Option<Design> {
    // Exact match first
    let result = designs
        .iter()
        .find(|d| d.id == id)
        // Fall back: query is an old un-namespaced id, match against suffix of stored ids
        .or_else(|| designs.iter().find(|d| d.id.rsplit_once(':').is_some_and(|(_, suffix)| suffix == id)))
        // Fall back: stored id is old un-namespaced, match against suffix of query
        .or_else(|| id.rsplit_once(':').and_then(|(_, suffix)| designs.iter().find(|d| d.id == suffix)))
        .cloned();
    if result.is_none() && !id.is_empty() {
        let available: Vec<&str> = designs.iter().map(|d| d.id.as_str()).collect();
        tracing::warn!("get_design: no match for id={id:?}, available={available:?}");
    }
    result
}

async fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> anyhow::Result<T> {
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
    Ok(serde_json::from_str(&json_str)?)
}
