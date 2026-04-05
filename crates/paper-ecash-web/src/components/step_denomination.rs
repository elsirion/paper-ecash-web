use leptos::prelude::*;

use crate::denomination::{self, MAX_SELECTIONS};

#[component]
pub fn StepDenomination(
    denominations_msat: RwSignal<Vec<u64>>,
    on_next: impl Fn() + Send + Sync + 'static,
    on_back: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    let available = denomination::available_denominations();
    let selected = RwSignal::new(denominations_msat.get_untracked());

    // Sync back to parent signal on every change
    let sync = move || {
        let sel = selected.get();
        denominations_msat.set(sel);
    };

    let total_msat = move || -> u64 { selected.get().iter().sum() };

    view! {
        <div>
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">"Denomination"</h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-6">
                "Select up to 4 denominations per paper note. Each denomination is a power-of-2 msat value."
            </p>

            <div class="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 gap-2 mb-6">
                {available
                    .into_iter()
                    .map(|denom| {
                        let is_selected = move || selected.get().contains(&denom);
                        let toggle = move |_| {
                            selected.update(|sel| {
                                if let Some(pos) = sel.iter().position(|&d| d == denom) {
                                    sel.remove(pos);
                                } else if sel.len() < MAX_SELECTIONS {
                                    sel.push(denom);
                                    sel.sort_unstable_by(|a, b| b.cmp(a));
                                }
                            });
                            sync();
                        };
                        view! {
                            <button
                                class=move || {
                                    if is_selected() {
                                        "px-3 py-2 text-sm font-semibold rounded-lg border-2 border-blue-500 bg-blue-600 text-white transition-all"
                                    } else {
                                        "px-3 py-2 text-sm font-semibold rounded-lg border-2 border-gray-200 dark:border-gray-600 bg-gray-50 dark:bg-gray-700 text-gray-900 dark:text-gray-200 hover:border-gray-400 dark:hover:border-gray-500 hover:-translate-y-0.5 transition-all"
                                    }
                                }
                                on:click=toggle
                            >
                                {denomination::format_amount_msat(denom)}
                            </button>
                        }
                    })
                    .collect::<Vec<_>>()}
            </div>

            <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-4 mb-6">
                <div class="flex justify-between text-sm py-1">
                    <span class="text-gray-500 dark:text-gray-400">"Selected:"</span>
                    <span class="text-gray-900 dark:text-white">{move || format!("{} / {MAX_SELECTIONS}", selected.get().len())}</span>
                </div>
                <div class="flex justify-between text-sm py-1 border-t border-gray-200 dark:border-gray-600 mt-1 pt-2 font-bold">
                    <span class="text-gray-900 dark:text-white">"Note value:"</span>
                    <span class="text-gray-900 dark:text-white">{move || denomination::format_amount_msat(total_msat())}</span>
                </div>
                {move || {
                    let sel = selected.get();
                    if sel.is_empty() {
                        None
                    } else {
                        Some(
                            view! {
                                <div class="flex gap-2 flex-wrap mt-3">
                                    {sel
                                        .iter()
                                        .map(|&d| {
                                            view! {
                                                <span class="bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300 text-xs font-semibold px-2.5 py-0.5 rounded-full">
                                                    {denomination::format_amount_msat(d)}
                                                </span>
                                            }
                                        })
                                        .collect::<Vec<_>>()}
                                </div>
                            },
                        )
                    }
                }}
            </div>

            <div class="flex flex-col-reverse sm:flex-row gap-3 sm:justify-end">
                <button
                    class="px-5 py-2.5 text-sm font-medium text-gray-900 bg-white border border-gray-300 rounded-lg hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:focus:ring-gray-700 transition-colors"
                    on:click=move |_| on_back()
                >
                    "Back"
                </button>
                <button
                    class="px-5 py-2.5 text-sm font-medium text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    disabled=move || selected.get().is_empty()
                    on:click=move |_| on_next()
                >
                    "Next"
                </button>
            </div>
        </div>
    }
}
