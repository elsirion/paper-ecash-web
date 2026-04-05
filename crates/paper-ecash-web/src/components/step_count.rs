use leptos::prelude::*;

use crate::denomination;

#[component]
pub fn StepCount(
    note_count: RwSignal<u32>,
    denominations_msat: RwSignal<Vec<u64>>,
    on_next: impl Fn() + Send + Sync + 'static,
    on_back: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    let per_note_msat = move || denominations_msat.get().iter().sum::<u64>();
    let total_msat = move || per_note_msat() * note_count.get() as u64;

    view! {
        <div>
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">"Note Count"</h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-6">"How many paper notes to generate?"</p>

            <div class="mb-4">
                <label for="count-input" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Number of notes"</label>
                <input
                    id="count-input"
                    type="number"
                    min="1"
                    max="1000"
                    class="block w-full p-2.5 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
                    prop:value=move || note_count.get().to_string()
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                            if v >= 1 {
                                note_count.set(v);
                            }
                        }
                    }
                />
            </div>

            <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-4 mb-6">
                <div class="flex justify-between text-sm py-1">
                    <span class="text-gray-500 dark:text-gray-400">"Per note:"</span>
                    <span class="text-gray-900 dark:text-white">
                        {move || denomination::format_amount_msat(per_note_msat())}
                    </span>
                </div>
                <div class="flex justify-between text-sm py-1">
                    <span class="text-gray-500 dark:text-gray-400">"Count:"</span>
                    <span class="text-gray-900 dark:text-white">{move || note_count.get().to_string()}</span>
                </div>
                <div class="flex justify-between text-sm py-1 border-t border-gray-200 dark:border-gray-600 mt-1 pt-2 font-bold">
                    <span class="text-gray-900 dark:text-white">"Total:"</span>
                    <span class="text-gray-900 dark:text-white">
                        {move || denomination::format_amount_msat(total_msat())}
                    </span>
                </div>
            </div>

            <div class="flex flex-col-reverse sm:flex-row gap-3 sm:justify-end">
                <button
                    class="px-5 py-2.5 text-sm font-medium text-gray-900 bg-white border border-gray-300 rounded-lg hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:focus:ring-gray-700 transition-colors"
                    on:click=move |_| on_back()
                >
                    "Back"
                </button>
                <button
                    class="px-5 py-2.5 text-sm font-medium text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 transition-colors"
                    on:click=move |_| on_next()
                >
                    "Next"
                </button>
            </div>
        </div>
    }
}
