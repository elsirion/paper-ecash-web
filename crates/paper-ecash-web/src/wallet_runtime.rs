use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

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

// ── Public event types ──

#[derive(Clone, Debug)]
pub enum OperationEvent {
    PaymentReceived { amount_msat: Option<u64> },
    SpendExactProgress { completed: u32, total: u32 },
    SpendExactDone { notes: Vec<String> },
    SpendExactFailed { error: String },
}

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

    pub async fn disconnect(&self) -> anyhow::Result<()> {
        self.worker.request(Command::Disconnect).await
    }

    pub async fn get_balance(&self) -> anyhow::Result<u64> {
        self.worker.request(Command::GetBalance).await
    }

    pub async fn get_federation_name(&self) -> anyhow::Result<String> {
        self.worker.request(Command::GetFederationName).await
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

    pub async fn watch_invoice(&self, operation_id: &str) -> anyhow::Result<()> {
        self.worker
            .request(Command::WatchInvoice {
                operation_id: operation_id.to_owned(),
            })
            .await
    }

    pub async fn spend_exact(
        &self,
        denominations_msat: Vec<u64>,
        include_invite: bool,
    ) -> anyhow::Result<Vec<String>> {
        self.worker
            .request(Command::SpendExact {
                denominations_msat,
                include_invite,
            })
            .await
    }

    pub fn set_event_listener(&self, listener: Option<Arc<dyn Fn(OperationEvent) + Send + Sync>>) {
        self.worker.set_event_listener(listener.map(|a| {
            let rc: Rc<dyn Fn(OperationEvent)> = Rc::new(move |e| a(e));
            rc
        }));
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
                let response = handle_request(runtime, scope.clone(), request).await;
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
    scope: DedicatedWorkerGlobalScope,
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
        Command::Disconnect => {
            runtime.borrow_mut().take();
            serialize_ok(())
        }
        Command::GetBalance => {
            with_runtime(&runtime, |wallet| async move {
                let balance = wallet.get_balance().await?;
                Ok(balance.msats)
            })
            .await
            .and_then(serialize_ok)
        }
        Command::GetFederationName => {
            with_runtime(&runtime, |wallet| async move {
                wallet.get_federation_name().await
            })
            .await
            .and_then(serialize_ok)
        }
        Command::CreateInvoice {
            amount_msat,
            description,
        } => {
            let scope = scope.clone();
            with_runtime(&runtime, move |wallet| async move {
                let (op_id_str, invoice_str) =
                    wallet.create_invoice(amount_msat, &description).await?;
                wallet.spawn_receive_watcher(op_id_str.clone(), move |amount_msat| {
                    let event = WireEvent::PaymentReceived { amount_msat };
                    let _ = post_message(&scope, &WorkerEventEnvelope { event });
                });
                Ok(InvoiceResponse {
                    operation_id: op_id_str,
                    invoice: invoice_str,
                })
            })
            .await
            .and_then(serialize_ok)
        }
        Command::WatchInvoice { operation_id } => {
            let scope = scope.clone();
            with_runtime(&runtime, move |wallet| async move {
                wallet.start_watching_invoice(&operation_id, move |amount_msat| {
                    let event = WireEvent::PaymentReceived { amount_msat };
                    let _ = post_message(&scope, &WorkerEventEnvelope { event });
                })?;
                Ok(())
            })
            .await
            .and_then(|_| serialize_ok(()))
        }
        Command::SpendExact {
            denominations_msat,
            include_invite,
        } => {
            with_runtime(&runtime, move |wallet| async move {
                wallet
                    .spend_exact(&denominations_msat, include_invite)
                    .await
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
type EventListener = Rc<dyn Fn(OperationEvent)>;

#[derive(Clone)]
struct WorkerClient {
    inner: Rc<WorkerClientInner>,
}

struct WorkerClientInner {
    worker: Worker,
    next_id: Cell<u64>,
    pending: Rc<RefCell<HashMap<u64, ResponseSender>>>,
    event_listener: Rc<RefCell<Option<EventListener>>>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl WorkerClient {
    fn new(worker: Worker) -> Self {
        let pending = Rc::new(RefCell::new(HashMap::<u64, ResponseSender>::new()));
        let event_listener = Rc::new(RefCell::new(None::<EventListener>));
        let on_message = Closure::wrap(Box::new({
            let pending = Rc::clone(&pending);
            let event_listener = Rc::clone(&event_listener);
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
                    return;
                }

                if let Ok(event_envelope) = serde_json::from_str::<WorkerEventEnvelope>(&raw) {
                    if let Some(callback) = event_listener.borrow().as_ref() {
                        callback(event_envelope.event.into_public());
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        let inner = WorkerClientInner {
            worker,
            next_id: Cell::new(1),
            pending,
            event_listener,
            _on_message: on_message,
        };
        Self {
            inner: Rc::new(inner),
        }
    }

    fn set_event_listener(&self, listener: Option<Rc<dyn Fn(OperationEvent)>>) {
        self.inner.event_listener.replace(listener);
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
    Disconnect,
    GetBalance,
    GetFederationName,
    CreateInvoice {
        amount_msat: u64,
        description: String,
    },
    WatchInvoice {
        operation_id: String,
    },
    SpendExact {
        denominations_msat: Vec<u64>,
        include_invite: bool,
    },
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

#[derive(Debug, Serialize, Deserialize)]
struct WorkerEventEnvelope {
    #[serde(flatten)]
    event: WireEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum WireEvent {
    PaymentReceived { amount_msat: Option<u64> },
    SpendExactProgress { completed: u32, total: u32 },
    SpendExactDone { notes: Vec<String> },
    SpendExactFailed { error: String },
}

impl WireEvent {
    fn into_public(self) -> OperationEvent {
        match self {
            Self::PaymentReceived { amount_msat } => {
                OperationEvent::PaymentReceived { amount_msat }
            }
            Self::SpendExactProgress { completed, total } => {
                OperationEvent::SpendExactProgress { completed, total }
            }
            Self::SpendExactDone { notes } => OperationEvent::SpendExactDone { notes },
            Self::SpendExactFailed { error } => OperationEvent::SpendExactFailed { error },
        }
    }
}
