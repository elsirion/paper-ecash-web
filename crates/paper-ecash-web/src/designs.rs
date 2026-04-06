use serde::Deserialize;
use wasm_bindgen::JsCast;

use crate::models::{QrErrorCorrection, TextConfig};

const DESIGNS_BASE_URL: &str =
    "https://raw.githubusercontent.com/elsiribot/paper-ecash-note-designs/main";

#[derive(Clone, Debug)]
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
    let index_url = format!("{DESIGNS_BASE_URL}/index.json");
    let ids: Vec<String> = fetch_json(&index_url).await?;

    let mut designs = Vec::with_capacity(ids.len());
    for id in ids {
        let design_url = format!("{DESIGNS_BASE_URL}/{id}/design.json");
        match fetch_json::<DesignJson>(&design_url).await {
            Ok(dj) => {
                let base = format!("{DESIGNS_BASE_URL}/{id}");
                designs.push(Design {
                    id: dj.id,
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
    designs.iter().find(|d| d.id == id).cloned()
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
