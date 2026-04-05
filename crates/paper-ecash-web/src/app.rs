use leptos::prelude::*;

use crate::components::design_editor::DesignEditor;
use crate::components::landing::Landing;
use crate::components::wizard::Wizard;
use crate::wallet_runtime::WalletRuntime;

#[derive(Clone, Debug, PartialEq)]
pub enum AppView {
    Landing,
    NewIssuance,
    ResumeIssuance(String),
    DesignEditor,
}

#[component]
pub fn App() -> impl IntoView {
    let view = RwSignal::new(AppView::Landing);
    let wallet: RwSignal<Option<WalletRuntime>> = RwSignal::new(None);
    let dark = RwSignal::new(is_dark_mode());

    // Spawn the worker once on mount
    let spawned = RwSignal::new(false);
    Effect::new(move || {
        if spawned.get_untracked() {
            return;
        }
        spawned.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match WalletRuntime::spawn().await {
                Ok(rt) => wallet.set(Some(rt)),
                Err(e) => tracing::error!("Failed to spawn wallet worker: {e:#}"),
            }
        });
    });

    let toggle_theme = move |_| {
        let new_dark = !dark.get_untracked();
        dark.set(new_dark);
        set_dark_mode(new_dark);
    };

    view! {
        <div class="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 pb-8">
            <header class="flex items-center justify-between py-4 border-b border-gray-200 dark:border-gray-700 mb-6">
                <h1
                    class="text-xl font-bold text-blue-600 dark:text-blue-400 cursor-pointer hover:text-blue-700 dark:hover:text-blue-300 transition-colors"
                    on:click=move |_| view.set(AppView::Landing)
                >
                    "Paper eCash"
                </h1>
                <div class="flex items-center gap-2">
                    <button
                        class="px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
                        on:click=move |_| view.set(AppView::DesignEditor)
                    >
                        "Design Editor"
                    </button>
                    <button
                        class="p-2 text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
                        on:click=toggle_theme
                        title="Toggle dark mode"
                    >
                        {move || if dark.get() {
                            view! {
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                                </svg>
                            }.into_any()
                        } else {
                            view! {
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
                                </svg>
                            }.into_any()
                        }}
                    </button>
                </div>
            </header>
            <main class="min-h-[60vh]">
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
                        AppView::DesignEditor => {
                            view! {
                                <DesignEditor
                                    on_back=move || view.set(AppView::Landing)
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

fn is_dark_mode() -> bool {
    let doc = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element());
    doc.map(|el| el.class_list().contains("dark")).unwrap_or(false)
}

fn set_dark_mode(dark: bool) {
    let window = web_sys::window().unwrap();
    if let Some(el) = window.document().and_then(|d| d.document_element()) {
        if dark {
            let _ = el.class_list().add_1("dark");
        } else {
            let _ = el.class_list().remove_1("dark");
        }
    }
    if let Ok(storage) = window.local_storage() {
        if let Some(storage) = storage {
            let _ = storage.set_item("theme", if dark { "dark" } else { "light" });
        }
    }
}
