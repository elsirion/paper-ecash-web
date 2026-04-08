use leptos::prelude::*;

use crate::designs::{self, Design, DesignSource, DEFAULT_DESIGNS_URL};
use crate::storage;

const LOCAL_SOURCE: &str = "local:";

type SourceGroup = (DesignSource, Vec<Design>);

#[component]
pub fn Landing(
    on_new: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    let source_groups: RwSignal<Vec<SourceGroup>> = RwSignal::new(Vec::new());
    let loading = RwSignal::new(false);

    // Load all sources on mount
    loading.set(true);
    wasm_bindgen_futures::spawn_local(async move {
        let mut groups: Vec<SourceGroup> = Vec::new();

        let default_source = DesignSource {
            name: "Default".into(),
            base_url: DEFAULT_DESIGNS_URL.into(),
        };
        if let Ok(d) = designs::fetch_designs_from(DEFAULT_DESIGNS_URL).await {
            groups.push((default_source, d));
        }

        let local_designs = storage::load_local_designs();
        if !local_designs.is_empty() {
            groups.push((
                DesignSource {
                    name: "Local (from Design Editor)".into(),
                    base_url: LOCAL_SOURCE.into(),
                },
                local_designs,
            ));
        }

        for source in storage::load_design_sources() {
            if let Ok(d) = designs::fetch_designs_from(&source.base_url).await {
                groups.push((source, d));
            }
        }

        source_groups.set(groups);
        loading.set(false);
    });

    view! {
        <div>
            <div class="flex flex-col items-center text-center mb-8">
                <p class="text-gray-500 dark:text-gray-400 text-sm sm:text-base mb-4">
                    "Generate printable paper ecash banknotes from fedimint"
                </p>
                <button
                    class="inline-flex items-center justify-center gap-2 px-8 py-4 text-base font-semibold text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 rounded-lg dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 transition-colors"
                    on:click=move |_| on_new()
                >
                    "Issue Paper Ecash"
                </button>
            </div>

            <div>
                <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">"Available Designs"</h2>
                {move || {
                    if loading.get() && source_groups.get().is_empty() {
                        return view! {
                            <div class="text-sm text-gray-500 dark:text-gray-400 text-center py-8">"Loading designs..."</div>
                        }.into_any();
                    }
                    let groups = source_groups.get();
                    if groups.is_empty() {
                        return view! {
                            <div class="text-sm text-gray-500 dark:text-gray-400 text-center py-8">"No designs found."</div>
                        }.into_any();
                    }
                    view! {
                        <div class="space-y-6">
                            {groups.into_iter().map(|(source, ds)| {
                                let source_name = source.name.clone();
                                view! {
                                    <div>
                                        <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-2">{source_name}</h3>
                                        {if ds.is_empty() {
                                            view! {
                                                <p class="text-xs text-gray-500 dark:text-gray-400 py-2">"No designs in this source."</p>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-3">
                                                    {ds.into_iter().map(|d| {
                                                        let name = d.name.clone();
                                                        let front_url = d.front_url.clone();
                                                        view! {
                                                            <div class="border-2 border-gray-200 dark:border-gray-700 rounded-lg p-2 text-center">
                                                                <img src=front_url alt=name.clone() class="w-full h-auto rounded mb-1" />
                                                                <span class="text-xs text-gray-600 dark:text-gray-400">{name}</span>
                                                            </div>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </div>
                                            }.into_any()
                                        }}
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}
