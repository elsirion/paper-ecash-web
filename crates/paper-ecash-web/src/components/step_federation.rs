use leptos::prelude::*;
use serde::Deserialize;
use wasm_bindgen::JsCast;

use crate::wallet_runtime::WalletRuntime;

const OBSERVER_API: &str = "https://observer.fedimint.org/api/federations";

#[derive(Clone, Debug, Deserialize)]
struct ObserverFederation {
    name: String,
    invite: String,
    health: String,
    #[serde(default)]
    deposits: u64,
}

#[component]
pub fn StepFederation(
    invite_code: RwSignal<String>,
    wallet: RwSignal<Option<WalletRuntime>>,
    federation_name: RwSignal<String>,
    on_next: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    let error = RwSignal::new(Option::<String>::None);
    let federations: RwSignal<Vec<ObserverFederation>> = RwSignal::new(Vec::new());
    let search = RwSignal::new(String::new());
    let show_manual = RwSignal::new(false);
    let loading_feds = RwSignal::new(true);

    // Fetch public federations on mount
    let fetched = RwSignal::new(false);
    Effect::new(move || {
        if fetched.get_untracked() {
            return;
        }
        fetched.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_federations().await {
                Ok(feds) => {
                    federations.set(feds);
                    loading_feds.set(false);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch federations: {e}");
                    loading_feds.set(false);
                    // Fall back to manual entry
                    show_manual.set(true);
                }
            }
        });
    });

    let on_next = std::sync::Arc::new(on_next);

    let select_federation = std::sync::Arc::new({
        let on_next = on_next.clone();
        move |fed: ObserverFederation| {
            invite_code.set(fed.invite);
            federation_name.set(fed.name);
            error.set(None);
            on_next();
        }
    });

    let validate_manual = std::sync::Arc::new({
        let on_next = on_next.clone();
        move || {
            let code = invite_code.get_untracked();
            if code.trim().is_empty() {
                error.set(Some("Please enter a federation invite code.".into()));
                return;
            }
            let trimmed = code.trim().to_string();
            if !trimmed.starts_with("fed1") {
                error.set(Some("Invalid invite code format. Should start with 'fed1'.".into()));
                return;
            }
            error.set(None);
            invite_code.set(trimmed);
            federation_name.set(String::new());
            on_next();
        }
    });

    let filtered_feds = move || {
        let query = search.get().to_lowercase();
        let feds = federations.get();
        if query.is_empty() {
            feds
        } else {
            feds.into_iter()
                .filter(|f| f.name.to_lowercase().contains(&query))
                .collect()
        }
    };

    view! {
        <div class="step">
            <h2>"Federation"</h2>
            <p class="step-description">
                "Select a public federation or enter an invite code manually."
            </p>

            {move || {
                let validate_manual = validate_manual.clone();
                let select_federation = select_federation.clone();
                if show_manual.get() {
                    view! {
                        <div class="manual-entry">
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
                                    .map(|e| view! { <div class="error-message">{e}</div> })
                            }}

                            <div class="step-actions">
                                <button
                                    class="btn btn-secondary"
                                    on:click=move |_| show_manual.set(false)
                                >
                                    "Back to list"
                                </button>
                                <button
                                    class="btn btn-primary"
                                    on:click={
                                        let validate_manual = validate_manual.clone();
                                        move |_| validate_manual()
                                    }
                                >
                                    "Next"
                                </button>
                            </div>
                        </div>
                    }
                        .into_any()
                } else {
                    view! {
                        <div class="federation-picker">
                            <div class="form-group">
                                <input
                                    type="text"
                                    class="input"
                                    placeholder="Search federations..."
                                    prop:value=move || search.get()
                                    on:input=move |ev| search.set(event_target_value(&ev))
                                />
                            </div>

                            {move || {
                                if loading_feds.get() {
                                    view! {
                                        <div class="status-message">"Loading federations..."</div>
                                    }
                                        .into_any()
                                } else {
                                    let feds = filtered_feds();
                                    if feds.is_empty() {
                                        view! {
                                            <div class="empty-state">
                                                <p>"No federations found."</p>
                                            </div>
                                        }
                                            .into_any()
                                    } else {
                                        view! {
                                            <div class="federation-list">
                                                {feds
                                                    .into_iter()
                                                    .map({
                                                        let select_federation = select_federation.clone();
                                                        move |fed| {
                                                        let fed_clone = fed.clone();
                                                        let select_federation = select_federation.clone();
                                                        let name = fed.name.clone();
                                                        let health = fed.health.clone();
                                                        let deposits_btc = fed.deposits as f64
                                                            / 100_000_000_000.0;
                                                        let health_class = if health == "online" {
                                                            "fed-health online"
                                                        } else {
                                                            "fed-health offline"
                                                        };
                                                        view! {
                                                            <button
                                                                class="federation-item"
                                                                on:click=move |_| {
                                                                    select_federation(fed_clone.clone())
                                                                }
                                                            >
                                                                <div class="fed-info">
                                                                    <span class="fed-name">
                                                                        {name}
                                                                    </span>
                                                                    <span class=health_class>
                                                                        {health}
                                                                    </span>
                                                                </div>
                                                                <div class="fed-detail">
                                                                    {format!("{deposits_btc:.4} BTC")}
                                                                </div>
                                                            </button>
                                                        }
                                                    }})
                                                    .collect::<Vec<_>>()}
                                            </div>
                                        }
                                            .into_any()
                                    }
                                }
                            }}

                            <div class="step-actions">
                                <button
                                    class="btn btn-secondary"
                                    on:click=move |_| show_manual.set(true)
                                >
                                    "Enter code manually"
                                </button>
                            </div>
                        </div>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}

async fn fetch_federations() -> anyhow::Result<Vec<ObserverFederation>> {
    let window = web_sys::window().ok_or_else(|| anyhow::anyhow!("no window"))?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(OBSERVER_API))
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| anyhow::anyhow!("not a Response"))?;
    if !resp.ok() {
        anyhow::bail!("HTTP {}", resp.status());
    }
    let text = wasm_bindgen_futures::JsFuture::from(
        resp.text().map_err(|e| anyhow::anyhow!("{e:?}"))?,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let json_str = text
        .as_string()
        .ok_or_else(|| anyhow::anyhow!("not a string"))?;
    let feds: Vec<ObserverFederation> = serde_json::from_str(&json_str)?;
    Ok(feds)
}
