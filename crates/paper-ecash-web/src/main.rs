#[cfg(target_family = "wasm")]
mod app;
#[cfg(target_family = "wasm")]
mod browser;
mod denomination;
mod designs;
#[cfg(target_family = "wasm")]
mod fedimint;
#[cfg(target_family = "wasm")]
mod fonts;
mod models;
mod pdf;
mod qr;
#[cfg(target_family = "wasm")]
mod storage;
#[cfg(target_family = "wasm")]
mod wallet_runtime;
#[cfg(target_family = "wasm")]
mod components;

#[cfg(target_family = "wasm")]
fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
    if wallet_runtime::run_worker_entrypoint() {
        return;
    }
    // Remove the static loading indicator now that WASM is ready
    if let Some(el) = leptos::prelude::document().get_element_by_id("loading") {
        el.remove();
    }
    leptos::mount::mount_to_body(app::App);
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    println!("Paper eCash Web is a wasm-only app. Use `trunk serve` or `trunk build`.");
}
