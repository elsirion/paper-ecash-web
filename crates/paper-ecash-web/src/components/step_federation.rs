use leptos::prelude::*;
use serde::Deserialize;
use wasm_bindgen::JsCast;

use crate::wallet_runtime::WalletRuntime;

const OBSERVER_API: &str = "https://observer.fedimint.org/api/federations";

#[derive(Clone, Debug, Deserialize)]
struct NostrVotes {
    count: u32,
    avg: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct ObserverFederation {
    name: String,
    invite: String,
    health: String,
    #[serde(default)]
    nostr_votes: Option<NostrVotes>,
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
            if !trimmed.starts_with("fed1") && !trimmed.starts_with("fedimint") {
                error.set(Some("Invalid invite code format.".into()));
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
        federations
            .get()
            .into_iter()
            .filter(|f| f.health == "online")
            .filter(|f| query.is_empty() || f.name.to_lowercase().contains(&query))
            .collect::<Vec<_>>()
    };

    view! {
        <div>
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">"Federation"</h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mb-6">
                "Select a public federation or enter an invite code manually."
            </p>

            {move || {
                let validate_manual = validate_manual.clone();
                let select_federation = select_federation.clone();
                if show_manual.get() {
                    view! {
                        <div>
                            <div class="mb-4">
                                <label for="invite-code" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Invite Code"</label>
                                <textarea
                                    id="invite-code"
                                    rows="3"
                                    class="block w-full p-2.5 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
                                    placeholder="fed1... or fedimint..."
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
                                    .map(|e| view! {
                                        <div class="p-4 mb-4 text-sm text-red-800 rounded-lg bg-red-50 dark:bg-gray-800 dark:text-red-400 border-l-4 border-red-500">{e}</div>
                                    })
                            }}

                            <div class="flex flex-col-reverse sm:flex-row gap-3 sm:justify-end">
                                <button
                                    class="px-5 py-2.5 text-sm font-medium text-gray-900 bg-white border border-gray-300 rounded-lg hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:focus:ring-gray-700 transition-colors"
                                    on:click=move |_| show_manual.set(false)
                                >
                                    "Back to list"
                                </button>
                                <button
                                    class="px-5 py-2.5 text-sm font-medium text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 transition-colors"
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
                        <div>
                            <div class="mb-4">
                                <input
                                    type="text"
                                    class="block w-full p-2.5 text-sm text-gray-900 bg-gray-50 rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
                                    placeholder="Search federations..."
                                    prop:value=move || search.get()
                                    on:input=move |ev| search.set(event_target_value(&ev))
                                />
                            </div>

                            {move || {
                                if loading_feds.get() {
                                    view! {
                                        <div class="text-sm text-gray-500 dark:text-gray-400 text-center py-8">"Loading federations..."</div>
                                    }
                                        .into_any()
                                } else {
                                    let feds = filtered_feds();
                                    if feds.is_empty() {
                                        view! {
                                            <div class="text-center py-8 text-gray-500 dark:text-gray-400">
                                                <p>"No federations found."</p>
                                            </div>
                                        }
                                            .into_any()
                                    } else {
                                        view! {
                                            <div class="flex flex-col gap-2 max-h-96 overflow-y-auto mb-4">
                                                {feds
                                                    .into_iter()
                                                    .map({
                                                        let select_federation = select_federation.clone();
                                                        move |fed| {
                                                        let fed_clone = fed.clone();
                                                        let select_federation = select_federation.clone();
                                                        let name = fed.name.clone();
                                                        let rating = fed.nostr_votes.as_ref().and_then(|v| {
                                                            v.avg.map(|avg| format!("{avg:.1}/5 ({} votes)", v.count))
                                                        }).unwrap_or_else(|| "no ratings".to_string());
                                                        view! {
                                                            <button
                                                                class="flex items-center justify-between w-full p-3 text-left bg-gray-50 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-lg hover:border-blue-500 dark:hover:border-blue-400 hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors"
                                                                on:click=move |_| {
                                                                    select_federation(fed_clone.clone())
                                                                }
                                                            >
                                                                <span class="font-medium text-gray-900 dark:text-white text-sm">
                                                                    {name}
                                                                </span>
                                                                <span class="text-xs text-gray-500 dark:text-gray-400 whitespace-nowrap ml-2">
                                                                    {rating}
                                                                </span>
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

                            <div class="flex justify-end">
                                <button
                                    class="px-5 py-2.5 text-sm font-medium text-gray-900 bg-white border border-gray-300 rounded-lg hover:bg-gray-100 focus:ring-4 focus:ring-gray-100 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:focus:ring-gray-700 transition-colors"
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
