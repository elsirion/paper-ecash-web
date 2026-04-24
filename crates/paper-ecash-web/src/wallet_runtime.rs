use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use anyhow::Context;
use futures::channel::oneshot;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent, Worker};

use crate::browser;
use crate::fedimint::WalletRuntimeCore;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvoiceResponse {
    pub operation_id: String,
    pub invoice: String,
}

// ── Main-thread API ──

#[derive(Clone)]
pub struct WalletRuntime {
    worker: send_wrapper::SendWrapper<WorkerClient>,
}

// Safety: WalletRuntime is only used on the main thread in WASM.
// SendWrapper ensures this at runtime.
unsafe impl Send for WorkerClient {}
unsafe impl Sync for WorkerClient {}

impl WalletRuntime {
    pub async fn spawn() -> anyhow::Result<Self> {
        let worker = browser::spawn_wallet_worker().await?;
        let client = WorkerClient::new(worker);
        Ok(Self {
            worker: send_wrapper::SendWrapper::new(client),
        })
    }

    pub async fn connect(
        &self,
        issuance_id: &str,
        mnemonic: &str,
        invite_code: &str,
    ) -> anyhow::Result<()> {
        self.worker
            .request(Command::Connect {
                issuance_id: issuance_id.to_owned(),
                mnemonic: mnemonic.to_owned(),
                invite_code: invite_code.to_owned(),
            })
            .await
    }

    pub async fn get_balance(&self) -> anyhow::Result<u64> {
        self.worker.request(Command::GetBalance).await
    }

    /// Returns `(base_msat, proportional_millionths)` for the gateway we'd use.
    pub async fn get_gateway_fees(&self) -> anyhow::Result<(u32, u32)> {
        self.worker.request(Command::GetGatewayFees).await
    }

    pub async fn create_invoice(
        &self,
        amount_msat: u64,
        description: &str,
    ) -> anyhow::Result<InvoiceResponse> {
        self.worker
            .request(Command::CreateInvoice {
                amount_msat,
                description: description.to_owned(),
            })
            .await
    }

    /// Scan the operation log for an existing LN receive operation.
    pub async fn find_ln_receive(&self) -> anyhow::Result<Option<InvoiceResponse>> {
        self.worker.request(Command::FindLnReceive).await
    }

    /// Subscribe to an existing LN receive and block until Claimed/Canceled.
    pub async fn wait_for_receive(&self, operation_id: &str) -> anyhow::Result<()> {
        self.worker
            .request(Command::WaitForReceive {
                operation_id: operation_id.to_owned(),
            })
            .await
    }

    /// Recover all issued OOB notes from the operation log.
    pub async fn recover_issued_notes(&self) -> anyhow::Result<Vec<String>> {
        self.worker.request(Command::RecoverIssuedNotes).await
    }

    pub async fn spend_exact(
        &self,
        denominations_msat: Vec<u64>,
    ) -> anyhow::Result<String> {
        self.worker
            .request(Command::SpendExact {
                denominations_msat,
            })
            .await
    }

    pub async fn pay_bolt11(&self, invoice: &str) -> anyhow::Result<()> {
        self.worker
            .request(Command::PayBolt11 {
                invoice: invoice.to_owned(),
            })
            .await
    }

    pub async fn reissue_notes(&self, notes: &str) -> anyhow::Result<()> {
        self.worker
            .request(Command::ReissueNotes {
                notes: notes.to_owned(),
            })
            .await
    }
}

// ── Worker entrypoint ──

pub fn run_worker_entrypoint() -> bool {
    if !browser::is_worker_context() {
        return false;
    }

    let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
    let runtime = Rc::new(RefCell::new(None::<(String, WalletRuntimeCore)>));
    let on_message = Closure::wrap(Box::new({
        let runtime = Rc::clone(&runtime);
        let scope = scope.clone();
        move |event: MessageEvent| {
            let Some(raw) = event.data().as_string() else {
                return;
            };

            let Ok(request) = serde_json::from_str::<RequestEnvelope>(&raw) else {
                return;
            };

            let runtime = Rc::clone(&runtime);
            let scope = scope.clone();
            spawn_local(async move {
                let response = handle_request(runtime, request).await;
                if let Err(err) = post_message(&scope, &response) {
                    let _ = post_message(
                        &scope,
                        &ResponseEnvelope {
                            id: response.id,
                            payload: ResponsePayload::Err {
                                message: format!("Failed to post worker response: {err}"),
                            },
                        },
                    );
                }
            });
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    scope.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();
    true
}

async fn handle_request(
    runtime: Rc<RefCell<Option<(String, WalletRuntimeCore)>>>,
    request: RequestEnvelope,
) -> ResponseEnvelope {
    let result = match request.command {
        Command::Connect {
            issuance_id,
            mnemonic,
            invite_code,
        } => {
            // Skip reconnection if already connected to this issuance
            let already_connected = runtime
                .borrow()
                .as_ref()
                .map(|(id, _)| id == &issuance_id)
                .unwrap_or(false);
            if already_connected {
                serialize_ok(())
            } else {
                // Drop any existing runtime
                runtime.borrow_mut().take();
                match WalletRuntimeCore::connect(&issuance_id, &mnemonic, &invite_code).await {
                    Ok(core) => {
                        runtime.borrow_mut().replace((issuance_id, core));
                        serialize_ok(())
                    }
                    Err(err) => Err(err),
                }
            }
        }
        Command::GetBalance => {
            with_runtime(&runtime, |wallet| async move {
                let balance = wallet.get_balance().await?;
                Ok(balance.msats)
            })
            .await
            .and_then(serialize_ok)
        }
        Command::GetGatewayFees => {
            with_runtime(&runtime, |wallet| async move {
                wallet.get_gateway_fees().await
            })
            .await
            .and_then(serialize_ok)
        }
        Command::CreateInvoice {
            amount_msat,
            description,
        } => {
            with_runtime(&runtime, move |wallet| async move {
                let (op_id_str, invoice_str) =
                    wallet.create_invoice(amount_msat, &description).await?;
                Ok(InvoiceResponse {
                    operation_id: op_id_str,
                    invoice: invoice_str,
                })
            })
            .await
            .and_then(serialize_ok)
        }
        Command::SpendExact {
            denominations_msat,
        } => {
            with_runtime(&runtime, move |wallet| async move {
                wallet
                    .spend_exact(&denominations_msat)
                    .await
            })
            .await
            .and_then(serialize_ok)
        }
        Command::PayBolt11 { invoice } => {
            with_runtime(&runtime, move |wallet| async move {
                wallet.pay_bolt11(&invoice).await
            })
            .await
            .and_then(|_| serialize_ok(()))
        }
        Command::ReissueNotes { notes } => {
            with_runtime(&runtime, move |wallet| async move {
                wallet.reissue_notes(&notes).await
            })
            .await
            .and_then(|_| serialize_ok(()))
        }
        Command::FindLnReceive => {
            with_runtime(&runtime, |wallet| async move {
                let result = wallet.find_ln_receive().await?;
                Ok(result.map(|(op_id, invoice)| InvoiceResponse {
                    operation_id: op_id,
                    invoice,
                }))
            })
            .await
            .and_then(serialize_ok)
        }
        Command::WaitForReceive { operation_id } => {
            with_runtime(&runtime, move |wallet| async move {
                wallet.wait_for_receive(&operation_id).await
            })
            .await
            .and_then(|_| serialize_ok(()))
        }
        Command::RecoverIssuedNotes => {
            with_runtime(&runtime, |wallet| async move {
                wallet.recover_issued_notes().await
            })
            .await
            .and_then(serialize_ok)
        }
    };

    ResponseEnvelope {
        id: request.id,
        payload: match result {
            Ok(value) => ResponsePayload::Ok { value },
            Err(err) => ResponsePayload::Err {
                message: format!("{err:#}"),
            },
        },
    }
}

async fn with_runtime<F, Fut, T>(
    runtime: &Rc<RefCell<Option<(String, WalletRuntimeCore)>>>,
    call: F,
) -> anyhow::Result<T>
where
    F: FnOnce(WalletRuntimeCore) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let wallet = runtime
        .borrow()
        .as_ref()
        .map(|(_, core)| core.clone())
        .ok_or_else(|| anyhow::anyhow!("Worker not connected. Call Connect first."))?;
    call(wallet).await
}

fn serialize_ok<T: Serialize>(value: T) -> anyhow::Result<serde_json::Value> {
    serde_json::to_value(value).map_err(Into::into)
}

fn post_message<T: Serialize>(
    scope: &DedicatedWorkerGlobalScope,
    message: &T,
) -> anyhow::Result<()> {
    let raw = serde_json::to_string(message)?;
    scope
        .post_message(&wasm_bindgen::JsValue::from_str(&raw))
        .map_err(|err| anyhow::anyhow!(format!("{err:?}")))
}

// ── Wire protocol types ──

type ResponseSender = oneshot::Sender<anyhow::Result<serde_json::Value>>;

#[derive(Clone)]
struct WorkerClient {
    inner: Rc<WorkerClientInner>,
}

struct WorkerClientInner {
    worker: Worker,
    next_id: Cell<u64>,
    pending: Rc<RefCell<HashMap<u64, ResponseSender>>>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl WorkerClient {
    fn new(worker: Worker) -> Self {
        let pending = Rc::new(RefCell::new(HashMap::<u64, ResponseSender>::new()));
        let on_message = Closure::wrap(Box::new({
            let pending = Rc::clone(&pending);
            move |event: MessageEvent| {
                let Some(raw) = event.data().as_string() else {
                    return;
                };

                if let Ok(envelope) = serde_json::from_str::<ResponseEnvelope>(&raw) {
                    let result = match envelope.payload {
                        ResponsePayload::Ok { value } => Ok(value),
                        ResponsePayload::Err { message } => Err(anyhow::anyhow!(message)),
                    };
                    if let Some(sender) = pending.borrow_mut().remove(&envelope.id) {
                        let _ = sender.send(result);
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        let inner = WorkerClientInner {
            worker,
            next_id: Cell::new(1),
            pending,
            _on_message: on_message,
        };
        Self {
            inner: Rc::new(inner),
        }
    }

    async fn request<T: DeserializeOwned>(&self, command: Command) -> anyhow::Result<T> {
        let id = self.inner.next_id.get();
        self.inner.next_id.set(id + 1);
        let (sender, receiver) = oneshot::channel();
        self.inner.pending.borrow_mut().insert(id, sender);

        let request = RequestEnvelope { id, command };
        let raw = serde_json::to_string(&request)?;
        if let Err(err) = self
            .inner
            .worker
            .post_message(&wasm_bindgen::JsValue::from_str(&raw))
        {
            self.inner.pending.borrow_mut().remove(&id);
            return Err(anyhow::anyhow!(format!("{err:?}")));
        }

        let value = receiver
            .await
            .context("Wallet worker response channel closed")??;
        Ok(serde_json::from_value(value)?)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum Command {
    Connect {
        issuance_id: String,
        mnemonic: String,
        invite_code: String,
    },
    GetBalance,
    GetGatewayFees,
    CreateInvoice {
        amount_msat: u64,
        description: String,
    },
    SpendExact {
        denominations_msat: Vec<u64>,
    },
    PayBolt11 {
        invoice: String,
    },
    ReissueNotes {
        notes: String,
    },
    FindLnReceive,
    WaitForReceive {
        operation_id: String,
    },
    RecoverIssuedNotes,
}

#[derive(Debug, Serialize, Deserialize)]
struct RequestEnvelope {
    id: u64,
    #[serde(flatten)]
    command: Command,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseEnvelope {
    id: u64,
    #[serde(flatten)]
    payload: ResponsePayload,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ResponsePayload {
    Ok { value: serde_json::Value },
    Err { message: String },
}
