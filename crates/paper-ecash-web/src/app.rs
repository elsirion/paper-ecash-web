use leptos::prelude::*;

use crate::components::landing::Landing;
use crate::components::wizard::Wizard;
use crate::wallet_runtime::WalletRuntime;

#[derive(Clone, Debug, PartialEq)]
pub enum AppView {
    Landing,
    NewIssuance,
    ResumeIssuance(String),
}

#[component]
pub fn App() -> impl IntoView {
    let view = RwSignal::new(AppView::Landing);
    let wallet: RwSignal<Option<WalletRuntime>> = RwSignal::new(None);

    // Spawn the worker once on mount
    Effect::new(move || {
        wasm_bindgen_futures::spawn_local(async move {
            match WalletRuntime::spawn().await {
                Ok(rt) => wallet.set(Some(rt)),
                Err(e) => tracing::error!("Failed to spawn wallet worker: {e:#}"),
            }
        });
    });

    view! {
        <div class="app">
            <header class="app-header">
                <h1
                    class="app-title"
                    on:click=move |_| view.set(AppView::Landing)
                >
                    "Paper eCash"
                </h1>
            </header>
            <main class="app-main">
                {move || {
                    let current_view = view.get();
                    match current_view {
                        AppView::Landing => {
                            view! {
                                <Landing
                                    on_new=move || view.set(AppView::NewIssuance)
                                    on_resume=move |id| view.set(AppView::ResumeIssuance(id))
                                />
                            }
                                .into_any()
                        }
                        AppView::NewIssuance => {
                            view! {
                                <Wizard
                                    wallet=wallet
                                    issuance_id=None
                                    on_done=move || view.set(AppView::Landing)
                                />
                            }
                                .into_any()
                        }
                        AppView::ResumeIssuance(id) => {
                            view! {
                                <Wizard
                                    wallet=wallet
                                    issuance_id=Some(id.clone())
                                    on_done=move || view.set(AppView::Landing)
                                />
                            }
                                .into_any()
                        }
                    }
                }}
            </main>
        </div>
    }
}
