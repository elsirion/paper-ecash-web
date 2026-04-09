use leptos::prelude::*;

use crate::designs::{self, Design, DesignSource, DEFAULT_DESIGNS_URL};
use crate::storage;

const LOCAL_SOURCE: &str = "local:";

type SourceGroup = (DesignSource, Vec<Design>);

#[component]
pub fn DesignsPage(
    on_new: impl Fn() + Send + Sync + Clone + 'static,
    on_edit: impl Fn(Design) + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let source_groups: RwSignal<Vec<SourceGroup>> = RwSignal::new(Vec::new());
    let loading = RwSignal::new(false);

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
            <div class="flex items-center justify-between mb-6">
                <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"Designs"</h2>
                <button
                    class="px-4 py-2 text-sm font-medium text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 transition-colors"
                    on:click={
                        let on_new = on_new.clone();
                        move |_| on_new()
                    }
                >
                    "Create New Design"
                </button>
            </div>

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
                let on_edit = on_edit.clone();
                view! {
                    <div class="space-y-8">
                        {groups.into_iter().map(|(source, ds)| {
                            let source_name = source.name.clone();
                            let on_edit = on_edit.clone();
                            view! {
                                <div>
                                    <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-3">{source_name}</h3>
                                    {if ds.is_empty() {
                                        view! {
                                            <p class="text-xs text-gray-500 dark:text-gray-400 py-2">"No designs in this source."</p>
                                        }.into_any()
                                    } else {
                                        let on_edit = on_edit.clone();
                                        view! {
                                            <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4">
                                                {ds.into_iter().map(|d| {
                                                    let name = d.name.clone();
                                                    let front_url = d.front_url.clone();
                                                    let back_url = d.back_url.clone();
                                                    let design_for_edit = d.clone();
                                                    let on_edit = on_edit.clone();
                                                    view! {
                                                        <div class="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
                                                            <div class="flex gap-1 p-2 bg-gray-50 dark:bg-gray-800">
                                                                <img src=front_url alt="front" class="w-1/2 h-auto rounded" />
                                                                <img src=back_url alt="back" class="w-1/2 h-auto rounded" />
                                                            </div>
                                                            <div class="flex items-center justify-between p-3">
                                                                <span class="text-sm font-medium text-gray-900 dark:text-white">{name}</span>
                                                                <button
                                                                    class="px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-lg transition-colors"
                                                                    on:click=move |_| on_edit(design_for_edit.clone())
                                                                >
                                                                    "Open in Editor"
                                                                </button>
                                                            </div>
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
    }
}
