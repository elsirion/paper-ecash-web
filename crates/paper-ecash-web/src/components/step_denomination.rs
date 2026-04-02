use leptos::prelude::*;

use crate::denomination;

#[component]
pub fn StepDenomination(
    denominations_msat: RwSignal<Vec<u64>>,
    on_next: impl Fn() + Send + Sync + 'static,
    on_back: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    let sats_input = RwSignal::new(String::from("1024"));
    let error = RwSignal::new(Option::<String>::None);
    let suggestions = RwSignal::new(Option::<(Option<u64>, Option<u64>)>::None);

    let update_decomposition = move || {
        let text = sats_input.get();
        let Ok(sats) = text.parse::<u64>() else {
            error.set(Some("Enter a valid number of sats.".into()));
            denominations_msat.set(Vec::new());
            suggestions.set(None);
            return;
        };
        if sats == 0 {
            error.set(Some("Amount must be greater than 0.".into()));
            denominations_msat.set(Vec::new());
            suggestions.set(None);
            return;
        }
        match denomination::decompose_sats(sats, 4) {
            Some(denoms) => {
                denominations_msat.set(denoms);
                error.set(None);
                suggestions.set(None);
            }
            None => {
                denominations_msat.set(Vec::new());
                let (lower, upper) = denomination::suggest_nearest_valid(sats, 4);
                suggestions.set(Some((lower, upper)));
                error.set(Some(format!(
                    "{sats} sats requires more than 4 denominations (too many set bits in binary)."
                )));
            }
        }
    };

    // Run initial decomposition
    update_decomposition();

    view! {
        <div class="step">
            <h2>"Denomination"</h2>
            <p class="step-description">
                "Set the value of each paper note. Must decompose into at most 4 power-of-2 denominations."
            </p>

            <div class="form-group">
                <label for="sats-input">"Sats per note"</label>
                <input
                    id="sats-input"
                    type="number"
                    min="1"
                    class="input"
                    prop:value=move || sats_input.get()
                    on:input=move |ev| {
                        sats_input.set(event_target_value(&ev));
                        update_decomposition();
                    }
                />
            </div>

            {move || {
                error
                    .get()
                    .map(|e| {
                        view! { <div class="error-message">{e}</div> }
                    })
            }}

            {move || {
                suggestions
                    .get()
                    .map(|(lower, upper)| {
                        view! {
                            <div class="suggestions">
                                <span>"Try: "</span>
                                {lower
                                    .map(|v| {
                                        view! {
                                            <button
                                                class="btn btn-sm btn-secondary"
                                                on:click=move |_| {
                                                    sats_input.set(v.to_string());
                                                    update_decomposition();
                                                }
                                            >
                                                {format!("{v} sats")}
                                            </button>
                                        }
                                    })}
                                {upper
                                    .map(|v| {
                                        view! {
                                            <button
                                                class="btn btn-sm btn-secondary"
                                                on:click=move |_| {
                                                    sats_input.set(v.to_string());
                                                    update_decomposition();
                                                }
                                            >
                                                {format!("{v} sats")}
                                            </button>
                                        }
                                    })}
                            </div>
                        }
                    })
            }}

            {move || {
                let denoms = denominations_msat.get();
                if denoms.is_empty() {
                    None
                } else {
                    Some(
                        view! {
                            <div class="denomination-breakdown">
                                <h3>"Breakdown"</h3>
                                <div class="denom-chips">
                                    {denoms
                                        .iter()
                                        .map(|&d| {
                                            view! {
                                                <span class="denom-chip">
                                                    {denomination::format_denomination_msat(d)}
                                                </span>
                                            }
                                        })
                                        .collect::<Vec<_>>()}
                                </div>
                            </div>
                        },
                    )
                }
            }}

            <div class="step-actions">
                <button class="btn btn-secondary" on:click=move |_| on_back()>
                    "Back"
                </button>
                <button
                    class="btn btn-primary"
                    disabled=move || denominations_msat.get().is_empty()
                    on:click=move |_| on_next()
                >
                    "Next"
                </button>
            </div>
        </div>
    }
}
