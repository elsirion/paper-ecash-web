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
    init_tracing();
    if wallet_runtime::run_worker_entrypoint() {
        return;
    }
    leptos::mount::mount_to_body(app::App);
}

#[cfg(target_family = "wasm")]
fn init_tracing() {
    use tracing::Level;
    use tracing_subscriber::filter::Targets;
    use tracing_subscriber::layer::SubscriberExt;

    // iroh logs NAT traversal retries every 100ms, which drowns everything
    // else in the console
    let filter = Targets::new()
        .with_default(Level::INFO)
        .with_target("paper_ecash_web", Level::DEBUG)
        .with_target("iroh", Level::WARN)
        .with_target("noq", Level::WARN);
    let console = tracing_wasm::WASMLayer::new(
        tracing_wasm::WASMLayerConfigBuilder::new()
            .set_report_logs_in_timings(false)
            .build(),
    );
    let subscriber = tracing_subscriber::registry().with(filter).with(console);
    tracing::subscriber::set_global_default(subscriber).expect("Tracing is only initialized once");
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    println!("Paper eCash Web is a wasm-only app. Use `trunk serve` or `trunk build`.");
}
