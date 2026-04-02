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
        <div class="step">
            <h2>"Note Count"</h2>
            <p class="step-description">"How many paper notes to generate?"</p>

            <div class="form-group">
                <label for="count-input">"Number of notes"</label>
                <input
                    id="count-input"
                    type="number"
                    min="1"
                    max="1000"
                    class="input"
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

            <div class="count-summary">
                <div class="summary-row">
                    <span>"Per note:"</span>
                    <span>
                        {move || denomination::format_denomination_msat(per_note_msat())}
                    </span>
                </div>
                <div class="summary-row">
                    <span>"Count:"</span>
                    <span>{move || note_count.get().to_string()}</span>
                </div>
                <div class="summary-row total">
                    <span>"Total:"</span>
                    <span>
                        {move || denomination::format_denomination_msat(total_msat())}
                    </span>
                </div>
            </div>

            <div class="step-actions">
                <button class="btn btn-secondary" on:click=move |_| on_back()>
                    "Back"
                </button>
                <button class="btn btn-primary" on:click=move |_| on_next()>
                    "Next"
                </button>
            </div>
        </div>
    }
}
