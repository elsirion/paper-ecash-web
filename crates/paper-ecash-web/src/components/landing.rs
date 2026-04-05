use std::sync::Arc;

use leptos::prelude::*;

use crate::models::Issuance;
use crate::storage;

#[component]
pub fn Landing(
    on_new: impl Fn() + Send + Sync + 'static,
    on_resume: impl Fn(String) + Send + Sync + 'static,
) -> impl IntoView {
    let issuances = RwSignal::new(storage::load_issuances());
    let on_resume = Arc::new(on_resume);

    let delete_issuance = move |id: String| {
        storage::delete_issuance(&id);
        issuances.set(storage::load_issuances());
    };

    view! {
        <div>
            <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 mb-6">
                <p class="text-gray-500 dark:text-gray-400 text-sm sm:text-base">
                    "Generate printable paper ecash banknotes from fedimint"
                </p>
                <button
                    class="inline-flex items-center justify-center gap-2 px-5 py-2.5 text-sm font-medium text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 rounded-lg dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 transition-colors whitespace-nowrap"
                    on:click=move |_| on_new()
                >
                    "+ New Issuance"
                </button>
            </div>

            <div>
                {move || {
                    let items = issuances.get();
                    if items.is_empty() {
                        view! {
                            <div class="text-center py-12 text-gray-500 dark:text-gray-400">
                                <p class="text-lg">"No issuances yet."</p>
                                <p class="text-sm mt-1">"Create one to get started."</p>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                                {items
                                    .into_iter()
                                    .map(|issuance| {
                                        let id_resume = issuance.id.clone();
                                        let id_delete = issuance.id.clone();
                                        let on_resume = on_resume.clone();
                                        view! {
                                            <IssuanceCard
                                                issuance=issuance
                                                on_click=move |_| on_resume(id_resume.clone())
                                                on_delete=move |_| delete_issuance(id_delete.clone())
                                            />
                                        }
                                    })
                                    .collect::<Vec<_>>()}
                            </div>
                        }
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn IssuanceCard(
    issuance: Issuance,
    on_click: impl Fn(web_sys::MouseEvent) + Send + Sync + 'static,
    on_delete: impl Fn(web_sys::MouseEvent) + Send + Sync + 'static,
) -> impl IntoView {
    let status_classes = match &issuance.status {
        crate::models::IssuanceStatus::AwaitingDeposit => {
            "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-300"
        }
        crate::models::IssuanceStatus::Funded => {
            "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300"
        }
        crate::models::IssuanceStatus::Issued => {
            "bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-300"
        }
        crate::models::IssuanceStatus::Complete => {
            "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300"
        }
    };

    view! {
        <div
            class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 cursor-pointer hover:border-blue-500 dark:hover:border-blue-400 hover:shadow-md transition-all"
            on:click=on_click
        >
            <div class="flex items-center justify-between mb-3">
                <span class="font-semibold text-gray-900 dark:text-white">{issuance.label.clone()}</span>
                <span class={format!("text-xs font-medium px-2.5 py-0.5 rounded-full {status_classes}")}>
                    {issuance.status.label()}
                </span>
            </div>
            <div class="grid grid-cols-2 gap-1 mb-3 text-sm">
                <div>
                    <span class="text-gray-500 dark:text-gray-400 mr-1">"Notes:"</span>
                    <span class="text-gray-900 dark:text-white">{issuance.config.note_count}</span>
                </div>
                <div>
                    <span class="text-gray-500 dark:text-gray-400 mr-1">"Per note:"</span>
                    <span class="text-gray-900 dark:text-white">{format!("{} sats", issuance.per_note_amount_sats())}</span>
                </div>
                <div>
                    <span class="text-gray-500 dark:text-gray-400 mr-1">"Total:"</span>
                    <span class="text-gray-900 dark:text-white">{format!("{} sats", issuance.total_amount_sats())}</span>
                </div>
                <div>
                    <span class="text-gray-500 dark:text-gray-400 mr-1">"Design:"</span>
                    <span class="text-gray-900 dark:text-white">{issuance.config.design_id.clone()}</span>
                </div>
            </div>
            <div class="flex justify-end">
                <button
                    class="px-3 py-1.5 text-xs font-medium text-red-700 dark:text-red-400 border border-red-300 dark:border-red-800 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors"
                    on:click=move |e: web_sys::MouseEvent| {
                        e.stop_propagation();
                        on_delete(e);
                    }
                >
                    "Delete"
                </button>
            </div>
        </div>
    }
}
