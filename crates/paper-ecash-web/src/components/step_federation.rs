use leptos::prelude::*;

use crate::wallet_runtime::WalletRuntime;

#[component]
pub fn StepFederation(
    invite_code: RwSignal<String>,
    wallet: RwSignal<Option<WalletRuntime>>,
    federation_name: RwSignal<String>,
    on_next: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);

    let validate_and_next = move || {
        let code = invite_code.get_untracked();
        if code.trim().is_empty() {
            error.set(Some("Please enter a federation invite code.".into()));
            return;
        }

        // Quick client-side validation: invite codes start with "fed1"
        let trimmed = code.trim().to_string();
        if !trimmed.starts_with("fed1") {
            error.set(Some("Invalid invite code format. Should start with 'fed1'.".into()));
            return;
        }

        error.set(None);
        invite_code.set(trimmed);

        // Federation name will be resolved when we actually connect in the deposit step
        federation_name.set(String::new());
        on_next();
    };

    view! {
        <div class="step">
            <h2>"Federation"</h2>
            <p class="step-description">"Enter the federation invite code for this issuance."</p>

            <div class="form-group">
                <label for="invite-code">"Invite Code"</label>
                <textarea
                    id="invite-code"
                    class="input"
                    rows="3"
                    placeholder="fed11..."
                    prop:value=move || invite_code.get()
                    on:input=move |ev| {
                        invite_code.set(event_target_value(&ev));
                        error.set(None);
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

            <div class="step-actions">
                <button
                    class="btn btn-primary"
                    disabled=move || loading.get()
                    on:click=move |_| validate_and_next()
                >
                    "Next"
                </button>
            </div>
        </div>
    }
}
