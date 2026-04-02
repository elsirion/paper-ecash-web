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
        <div class="landing">
            <div class="landing-header">
                <p class="landing-subtitle">
                    "Generate printable paper ecash banknotes from fedimint"
                </p>
                <button class="btn btn-primary" on:click=move |_| on_new()>
                    "+ New Issuance"
                </button>
            </div>

            <div class="issuance-list">
                {move || {
                    let items = issuances.get();
                    if items.is_empty() {
                        view! {
                            <div class="empty-state">
                                <p>"No issuances yet. Create one to get started."</p>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="issuance-grid">
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
    let status_class = match &issuance.status {
        crate::models::IssuanceStatus::AwaitingDeposit => "status-awaiting",
        crate::models::IssuanceStatus::Funded => "status-funded",
        crate::models::IssuanceStatus::Issued => "status-issued",
        crate::models::IssuanceStatus::Complete => "status-complete",
    };

    view! {
        <div class="issuance-card" on:click=on_click>
            <div class="card-header">
                <span class="card-label">{issuance.label.clone()}</span>
                <span class={format!("card-status {status_class}")}>{issuance.status.label()}</span>
            </div>
            <div class="card-body">
                <div class="card-detail">
                    <span class="detail-label">"Notes:"</span>
                    <span>{issuance.config.note_count}</span>
                </div>
                <div class="card-detail">
                    <span class="detail-label">"Per note:"</span>
                    <span>{format!("{} sats", issuance.per_note_amount_sats())}</span>
                </div>
                <div class="card-detail">
                    <span class="detail-label">"Total:"</span>
                    <span>{format!("{} sats", issuance.total_amount_sats())}</span>
                </div>
                <div class="card-detail">
                    <span class="detail-label">"Design:"</span>
                    <span>{issuance.config.design_id.clone()}</span>
                </div>
            </div>
            <div class="card-footer">
                <button
                    class="btn btn-danger btn-sm"
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
