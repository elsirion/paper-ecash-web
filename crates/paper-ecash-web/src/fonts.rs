use serde::Deserialize;
use wasm_bindgen::JsCast;

use crate::browser;

/// Top Google Fonts by popularity. First 20 shown by default, rest searchable.
pub const GOOGLE_FONTS: &[&str] = &[
    "Roboto", "Open Sans", "Lato", "Montserrat", "Oswald",
    "Poppins", "Raleway", "Inter", "Nunito", "Ubuntu",
    "Playfair Display", "Merriweather", "PT Sans", "Roboto Condensed", "Roboto Mono",
    "Source Code Pro", "Bebas Neue", "Archivo Black", "Anton", "Bangers",
    "Noto Sans", "Noto Serif", "Rubik", "Work Sans", "Fira Sans",
    "Quicksand", "Barlow", "Mulish", "Kanit", "Inconsolata",
    "Titillium Web", "Heebo", "Libre Baskerville", "Libre Franklin", "Karla",
    "Manrope", "Josefin Sans", "DM Sans", "Cabin", "Arimo",
    "Bitter", "Exo 2", "Overpass", "Asap", "IBM Plex Sans",
    "IBM Plex Mono", "Crimson Text", "Yanone Kaffeesatz", "Abel", "Archivo",
    "Catamaran", "Signika", "Varela Round", "Questrial", "Rokkitt",
    "Fjalla One", "Jost", "Mukta", "Hind", "Cairo",
    "Cormorant Garamond", "Spectral", "Space Grotesk", "Space Mono", "Zilla Slab",
    "Prompt", "Sarabun", "Public Sans", "Outfit", "Sora",
    "Lexend", "Plus Jakarta Sans", "Red Hat Display", "Zen Kaku Gothic New", "Nanum Gothic",
    "Comfortaa", "Pacifico", "Permanent Marker", "Satisfy", "Lobster",
    "Dancing Script", "Great Vibes", "Caveat", "Shadows Into Light", "Sacramento",
    "Amatic SC", "Righteous", "Bungee", "Press Start 2P", "VT323",
    "Silkscreen", "Pixelify Sans", "Share Tech Mono", "JetBrains Mono", "Fira Code",
    "Source Sans 3", "Noto Sans Mono", "PT Serif", "EB Garamond", "Lora",
];

/// Inject a Google Fonts CSS link into the document head for preview rendering.
pub fn inject_font_link(family: &str) {
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };
    let Some(head) = document.head() else { return };
    let link_id = format!("gfont-{}", family.replace(' ', "-"));
    if document.get_element_by_id(&link_id).is_some() {
        return;
    }
    let Ok(link) = document.create_element("link") else { return };
    let _ = link.set_attribute("id", &link_id);
    let _ = link.set_attribute("rel", "stylesheet");
    let _ = link.set_attribute(
        "href",
        &format!(
            "https://fonts.googleapis.com/css2?family={}&display=swap",
            js_sys::encode_uri_component(family)
        ),
    );
    let _ = head.append_child(&link);
}

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
