use std::sync::Arc;

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
    let on_next = Arc::new(on_next);

    let validate_and_next = move || {
        let on_next = on_next.clone();
        let code = invite_code.get_untracked();
        if code.trim().is_empty() {
            error.set(Some("Please enter a federation invite code.".into()));
            return;
        }
        error.set(None);

        // Try to parse the invite code by connecting temporarily
        let Some(rt) = wallet.get_untracked() else {
            error.set(Some("Wallet worker not ready yet. Please wait.".into()));
            return;
        };

        loading.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            // Validate the invite code by trying to connect
            // We use a dummy issuance ID just for validation
            match rt
                .connect(
                    "__validation__",
                    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
                    &code,
                )
                .await
            {
                Ok(()) => {
                    // Get federation name
                    match rt.get_federation_name().await {
                        Ok(name) => federation_name.set(name),
                        Err(_) => federation_name.set("Unknown".into()),
                    }
                    let _ = rt.disconnect().await;
                    loading.set(false);
                    on_next();
                }
                Err(e) => {
                    loading.set(false);
                    error.set(Some(format!("Invalid invite code: {e}")));
                }
            }
        });
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

            {move || {
                let name = federation_name.get();
                if !name.is_empty() {
                    Some(
                        view! {
                            <div class="federation-info">
                                <span class="detail-label">"Federation: "</span>
                                <span>{name}</span>
                            </div>
                        },
                    )
                } else {
                    None
                }
            }}

            <div class="step-actions">
                <button
                    class="btn btn-primary"
                    disabled=move || loading.get()
                    on:click=move |_| validate_and_next()
                >
                    {move || if loading.get() { "Connecting..." } else { "Next" }}
                </button>
            </div>
        </div>
    }
}
