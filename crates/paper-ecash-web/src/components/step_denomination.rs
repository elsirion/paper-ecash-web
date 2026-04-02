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
        <div class="step">
            <h2>"Denomination"</h2>
            <p class="step-description">
                "Select up to 4 denominations per paper note. Each denomination is a power-of-2 msat value."
            </p>

            <div class="denom-grid">
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
                                        "denom-btn selected"
                                    } else {
                                        "denom-btn"
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

            <div class="denom-summary">
                <div class="summary-row">
                    <span>"Selected:"</span>
                    <span>{move || format!("{} / {MAX_SELECTIONS}", selected.get().len())}</span>
                </div>
                <div class="summary-row total">
                    <span>"Note value:"</span>
                    <span>{move || denomination::format_amount_msat(total_msat())}</span>
                </div>
                {move || {
                    let sel = selected.get();
                    if sel.is_empty() {
                        None
                    } else {
                        Some(
                            view! {
                                <div class="denom-chips">
                                    {sel
                                        .iter()
                                        .map(|&d| {
                                            view! {
                                                <span class="denom-chip">
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

            <div class="step-actions">
                <button class="btn btn-secondary" on:click=move |_| on_back()>
                    "Back"
                </button>
                <button
                    class="btn btn-primary"
                    disabled=move || selected.get().is_empty()
                    on:click=move |_| on_next()
                >
                    "Next"
                </button>
            </div>
        </div>
    }
}
