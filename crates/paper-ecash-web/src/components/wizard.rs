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
    let design_id = RwSignal::new(String::from("fedi"));
    let denominations_msat: RwSignal<Vec<u64>> = RwSignal::new(Vec::new());
    let note_count = RwSignal::new(1u32);
    let qr_x_offset = RwSignal::new(0.0f64);
    let qr_y_offset = RwSignal::new(0.0f64);
    let qr_size = RwSignal::new(7.0f64);
    let qr_ec = RwSignal::new(QrErrorCorrection::M);

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

            let resume_step = match existing.status {
                IssuanceStatus::AwaitingDeposit => WizardStep::Deposit,
                IssuanceStatus::Funded => WizardStep::Issue,
                IssuanceStatus::Issued | IssuanceStatus::Complete => WizardStep::Pdf,
            };
            step.set(resume_step);
            issuance.set(Some(existing));
        }
    }

    let build_config = move || IssuanceConfig {
        federation_invite: invite_code.get_untracked(),
        design_id: design_id.get_untracked(),
        denominations_msat: denominations_msat.get_untracked(),
        note_count: note_count.get_untracked(),
        qr_x_offset_cm: qr_x_offset.get_untracked(),
        qr_y_offset_cm: qr_y_offset.get_untracked(),
        qr_size_cm: qr_size.get_untracked(),
        qr_error_correction: qr_ec.get_untracked(),
    };

    let step_names = [
        ("Federation", WizardStep::Federation),
        ("Design", WizardStep::Design),
        ("Denomination", WizardStep::Denomination),
        ("Count", WizardStep::Count),
        ("Deposit", WizardStep::Deposit),
        ("Issue", WizardStep::Issue),
        ("PDF", WizardStep::Pdf),
    ];

    view! {
        <div class="wizard">
            <nav class="wizard-steps">
                {step_names
                    .iter()
                    .map(|(name, s)| {
                        let s = s.clone();
                        let current = step.clone();
                        view! {
                            <span class=move || {
                                if current.get() == s { "step-indicator active" } else { "step-indicator" }
                            }>{*name}</span>
                        }
                    })
                    .collect::<Vec<_>>()}
            </nav>

            <div class="wizard-content">
                {move || {
                    let on_done = on_done.clone();
                    match step.get() {
                        WizardStep::Federation => {
                            view! {
                                <StepFederation
                                    invite_code=invite_code
                                    wallet=wallet
                                    federation_name=federation_name
                                    on_next=move || step.set(WizardStep::Design)
                                />
                            }
                                .into_any()
                        }
                        WizardStep::Design => {
                            view! {
                                <StepDesign
                                    designs=designs
                                    design_id=design_id
                                    qr_x_offset=qr_x_offset
                                    qr_y_offset=qr_y_offset
                                    qr_size=qr_size
                                    qr_ec=qr_ec
                                    on_next=move || step.set(WizardStep::Denomination)
                                    on_back=move || step.set(WizardStep::Federation)
                                />
                            }
                                .into_any()
                        }
                        WizardStep::Denomination => {
                            view! {
                                <StepDenomination
                                    denominations_msat=denominations_msat
                                    on_next=move || step.set(WizardStep::Count)
                                    on_back=move || step.set(WizardStep::Design)
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
                                    on_next=move || step.set(WizardStep::Pdf)
                                />
                            }
                                .into_any()
                        }
                        WizardStep::Pdf => {
                            view! { <StepPdf issuance=issuance designs=designs on_done=on_done.clone() /> }
                                .into_any()
                        }
                    }
                }}
            </div>
        </div>
    }
}
