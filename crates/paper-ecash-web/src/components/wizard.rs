use std::sync::Arc;

use leptos::prelude::*;

use crate::components::step_count::StepCount;
use crate::components::step_denomination::StepDenomination;
use crate::components::step_deposit::StepDeposit;
use crate::components::step_design::StepDesign;
use crate::components::step_federation::StepFederation;
use crate::components::step_issue::StepIssue;
use crate::components::step_pdf::StepPdf;
use crate::designs::Design;
use crate::models::{Issuance, IssuanceConfig, IssuanceStatus, QrErrorCorrection};
use crate::storage;
use crate::wallet_runtime::WalletRuntime;

#[derive(Clone, Debug, PartialEq)]
pub enum WizardStep {
    Federation,
    Design,
    Denomination,
    Count,
    Deposit,
    Issue,
    Pdf,
}

#[component]
pub fn Wizard(
    wallet: RwSignal<Option<WalletRuntime>>,
    issuance_id: Option<String>,
    on_done: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let step = RwSignal::new(WizardStep::Federation);
    let issuance: RwSignal<Option<Issuance>> = RwSignal::new(None);
    let designs: RwSignal<Vec<Design>> = RwSignal::new(Vec::new());

    // Fetch designs
    let designs_fetched = RwSignal::new(false);
    Effect::new(move || {
        if designs_fetched.get_untracked() {
            return;
        }
        designs_fetched.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match crate::designs::fetch_designs().await {
                Ok(d) => designs.set(d),
                Err(e) => tracing::error!("Failed to fetch designs: {e:#}"),
            }
        });
    });

    // Wizard config signals
    let invite_code = RwSignal::new(String::new());
    let federation_name = RwSignal::new(String::new());
    let design_id = RwSignal::new(String::new());
    let denominations_msat: RwSignal<Vec<u64>> = RwSignal::new(Vec::new());
    let note_count = RwSignal::new(1u32);
    let qr_x_offset = RwSignal::new(0.0f64);
    let qr_y_offset = RwSignal::new(0.0f64);
    let qr_size = RwSignal::new(7.0f64);
    let qr_ec = RwSignal::new(QrErrorCorrection::M);

    // Amount text: only the literal text content. Placement/font come from the design.
    let text_sample = RwSignal::new(String::new());

    // If resuming, load the issuance and set signals + step
    if let Some(id) = &issuance_id {
        if let Some(existing) = storage::load_issuance(id) {
            invite_code.set(existing.config.federation_invite.clone());
            design_id.set(existing.config.design_id.clone());
            denominations_msat.set(existing.config.denominations_msat.clone());
            note_count.set(existing.config.note_count);
            qr_x_offset.set(existing.config.qr_x_offset_cm);
            qr_y_offset.set(existing.config.qr_y_offset_cm);
            qr_size.set(existing.config.qr_size_cm);
            qr_ec.set(existing.config.qr_error_correction);
            if let Some(at) = &existing.config.amount_text {
                if let Some(t) = &at.text {
                    text_sample.set(t.clone());
                }
            }

            // Design is now chosen after issuance, so resuming after Issued lands
            // on the Design step (which lets the user re-pick a design even for
            // already-complete issuances).
            let resume_step = match existing.status {
                IssuanceStatus::AwaitingDeposit => WizardStep::Deposit,
                IssuanceStatus::Funded => WizardStep::Issue,
                IssuanceStatus::Issued | IssuanceStatus::Complete => WizardStep::Design,
            };
            step.set(resume_step);
            issuance.set(Some(existing));
        }
    }

    let build_config = move || {
        let did = design_id.get_untracked();
        // Clone the design's amount_text and override its text with the user's input
        let amount_text = crate::designs::get_design(&designs.get_untracked(), &did)
            .and_then(|d| d.amount_text)
            .map(|mut at| {
                let sample = text_sample.get_untracked();
                at.text = if sample.trim().is_empty() { None } else { Some(sample) };
                at
            });
        IssuanceConfig {
            federation_invite: invite_code.get_untracked(),
            design_id: did,
            denominations_msat: denominations_msat.get_untracked(),
            note_count: note_count.get_untracked(),
            qr_x_offset_cm: qr_x_offset.get_untracked(),
            qr_y_offset_cm: qr_y_offset.get_untracked(),
            qr_size_cm: qr_size.get_untracked(),
            qr_error_correction: qr_ec.get_untracked(),
            amount_text,
        }
    };

    let step_names = [
        ("Federation", WizardStep::Federation),
        ("Denomination", WizardStep::Denomination),
        ("Count", WizardStep::Count),
        ("Deposit", WizardStep::Deposit),
        ("Issue", WizardStep::Issue),
        ("Design", WizardStep::Design),
        ("PDF", WizardStep::Pdf),
    ];

    // When leaving the Design step, persist the updated config to the issuance.
    let save_design_to_issuance = {
        let build_config = build_config.clone();
        move || {
            if let Some(mut iss) = issuance.get_untracked() {
                iss.config = build_config();
                storage::save_issuance(&iss);
                issuance.set(Some(iss));
            }
        }
    };

    view! {
        <div>
            <nav class="flex flex-wrap gap-2 mb-6">
                {step_names
                    .iter()
                    .map(|(name, s)| {
                        let s = s.clone();
                        let current = step.clone();
                        view! {
                            <span class=move || {
                                if current.get() == s {
                                    "text-xs font-medium px-3 py-1.5 rounded-full bg-blue-600 text-white"
                                } else {
                                    "text-xs font-medium px-3 py-1.5 rounded-full bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400"
                                }
                            }>{*name}</span>
                        }
                    })
                    .collect::<Vec<_>>()}
            </nav>

            <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 sm:p-6">
                {move || {
                    let on_done = on_done.clone();
                    match step.get() {
                        WizardStep::Federation => {
                            view! {
                                <StepFederation
                                    invite_code=invite_code
                                    wallet=wallet
                                    federation_name=federation_name
                                    on_next=move || step.set(WizardStep::Denomination)
                                />
                            }
                                .into_any()
                        }
                        WizardStep::Denomination => {
                            view! {
                                <StepDenomination
                                    denominations_msat=denominations_msat
                                    on_next=move || step.set(WizardStep::Count)
                                    on_back=move || step.set(WizardStep::Federation)
                                />
                            }
                                .into_any()
                        }
                        WizardStep::Count => {
                            view! {
                                <StepCount
                                    note_count=note_count
                                    denominations_msat=denominations_msat
                                    on_next=move || step.set(WizardStep::Deposit)
                                    on_back=move || step.set(WizardStep::Denomination)
                                />
                            }
                                .into_any()
                        }
                        WizardStep::Deposit => {
                            let build_config = build_config.clone();
                            view! {
                                <StepDeposit
                                    wallet=wallet
                                    issuance=issuance
                                    build_config=Arc::new(build_config)
                                    federation_name=federation_name
                                    on_next=move || step.set(WizardStep::Issue)
                                />
                            }
                                .into_any()
                        }
                        WizardStep::Issue => {
                            view! {
                                <StepIssue
                                    wallet=wallet
                                    issuance=issuance
                                    on_next=move || step.set(WizardStep::Design)
                                />
                            }
                                .into_any()
                        }
                        WizardStep::Design => {
                            let save_design_to_issuance = save_design_to_issuance.clone();
                            view! {
                                <StepDesign
                                    designs=designs
                                    design_id=design_id
                                    qr_x_offset=qr_x_offset
                                    qr_y_offset=qr_y_offset
                                    qr_size=qr_size
                                    qr_ec=qr_ec
                                    text_sample=text_sample
                                    on_next=move || {
                                        save_design_to_issuance();
                                        step.set(WizardStep::Pdf);
                                    }
                                    on_back=move || step.set(WizardStep::Issue)
                                />
                            }
                                .into_any()
                        }
                        WizardStep::Pdf => {
                            view! {
                                <StepPdf
                                    issuance=issuance
                                    designs=designs
                                    on_done=on_done.clone()
                                    on_back=move || step.set(WizardStep::Design)
                                />
                            }.into_any()
                        }
                    }
                }}
            </div>
        </div>
    }
}
