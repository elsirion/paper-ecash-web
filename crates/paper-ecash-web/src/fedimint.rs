use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;

use fedimint_bip39::{Bip39RootSecretStrategy, Language, Mnemonic};
use fedimint_client::meta::MetaService;
use fedimint_client::secret::RootSecretStrategy;
use fedimint_client::{Client, ClientHandle, RootSecret};
use fedimint_connectors::ConnectorRegistry;
use fedimint_core::core::OperationId;
use fedimint_core::db::mem_impl::MemDatabase;
use fedimint_core::db::Database;
use fedimint_core::invite_code::InviteCode;
use fedimint_core::secp256k1::PublicKey;
use fedimint_core::{Amount, TieredCounts, TieredMulti};
use fedimint_cursed_redb::MemAndRedb;
use fedimint_derive_secret::{ChildId, DerivableSecret};
use fedimint_ln_client::{LightningClientInit, LightningClientModule, LnReceiveState};
use fedimint_meta_client::{MetaClientInit, MetaModuleMetaSourceWithFallback};
use fedimint_mint_client::{MintClientInit, MintClientModule, OOBNotes, SpendExactState};
use fedimint_wallet_client::WalletClientInit;
use futures::StreamExt;
use tracing::info;
use wasm_bindgen_futures::spawn_local;

use crate::browser;

#[derive(Clone)]
pub(crate) struct WalletRuntimeCore {
    database: Database,
    client: Rc<RefCell<Option<Rc<ClientHandle>>>>,
    invite_code: InviteCode,
    mnemonic: Mnemonic,
    connectors: ConnectorRegistry,
}

impl WalletRuntimeCore {
    pub async fn connect(
        issuance_id: &str,
        mnemonic_words: &str,
        invite_code_str: &str,
    ) -> anyhow::Result<Self> {
        let db_file = format!("issuance-{issuance_id}.redb");
        let invite_code = InviteCode::from_str(invite_code_str)?;
        let mnemonic = Mnemonic::parse_in_normalized(Language::English, mnemonic_words)?;

        let database = match browser::open_wallet_handle(&db_file).await {
            Ok(handle) => {
                let cursed_db = MemAndRedb::new(handle)?;
                Database::new(cursed_db, Default::default())
            }
            Err(err) => {
                if browser::supports_sync_access_handles() {
                    return Err(anyhow::anyhow!(
                        "OPFS storage init failed: {err}"
                    ));
                }
                info!("OPFS unavailable ({err}), using in-memory DB");
                Database::new(MemDatabase::new(), Default::default())
            }
        };

        let connectors = ConnectorRegistry::build_from_client_defaults()
            .bind()
            .await?;

        let runtime = Self {
            database,
            client: Rc::new(RefCell::new(None)),
            invite_code,
            mnemonic,
            connectors,
        };

        // Eagerly build/open the client
        runtime.ensure_client().await?;

        Ok(runtime)
    }

    pub async fn get_balance(&self) -> anyhow::Result<Amount> {
        let client = self.ensure_client().await?;
        client.get_balance_for_btc().await
    }

    pub async fn get_federation_name(&self) -> anyhow::Result<String> {
        let client = self.ensure_client().await?;
        let name = client
            .meta_service()
            .get_field::<String>(&client.db(), "federation_name")
            .await
            .and_then(|meta| meta.value);
        Ok(name.unwrap_or_else(|| "Unknown Federation".to_owned()))
    }

    pub async fn create_invoice(
        &self,
        amount_msat: u64,
        description: &str,
    ) -> anyhow::Result<(String, String)> {
        let client = self.ensure_client().await?;
        let ln = client.get_first_module::<LightningClientModule>()?.inner();
        let gateway = ln.get_gateway(None::<PublicKey>, false).await?;
        let amount = Amount::from_msats(amount_msat);
        let (operation_id, invoice, _) = ln
            .create_bolt11_invoice(
                amount,
                lightning_invoice::Bolt11InvoiceDescription::Direct(
                    lightning_invoice::Description::new(description.to_owned())?,
                ),
                None,
                (),
                gateway,
            )
            .await?;
        Ok((operation_id.fmt_full().to_string(), invoice.to_string()))
    }

    pub fn spawn_receive_watcher(
        &self,
        operation_id_str: String,
        on_received: impl FnOnce(Option<u64>) + 'static,
    ) {
        let this = self.clone();
        spawn_local(async move {
            let Ok(client) = this.ensure_client().await else {
                return;
            };
            let Ok(op_id) = parse_operation_id(&operation_id_str) else {
                return;
            };
            let Ok(ln) = client
                .get_first_module::<LightningClientModule>()
                .map(|m| m.inner())
            else {
                return;
            };
            let Ok(sub) = ln.subscribe_ln_receive(op_id).await else {
                return;
            };
            let mut stream = sub.into_stream();
            while let Some(state) = stream.next().await {
                match state {
                    LnReceiveState::Claimed => {
                        on_received(None);
                        return;
                    }
                    LnReceiveState::Canceled { .. } => return,
                    _ => {}
                }
            }
        });
    }

    pub fn start_watching_invoice(
        &self,
        operation_id_str: &str,
        on_received: impl FnOnce(Option<u64>) + 'static,
    ) -> anyhow::Result<()> {
        let op_id_str = operation_id_str.to_owned();
        self.spawn_receive_watcher(op_id_str, on_received);
        Ok(())
    }

    pub async fn spend_exact(
        &self,
        denominations_msat: &[u64],
        include_invite: bool,
    ) -> anyhow::Result<Vec<String>> {
        let client = self.ensure_client().await?;
        let mint = client.get_first_module::<MintClientModule>()?.inner();

        // Build TieredCounts from the denomination list
        let mut tiered_counts = TieredCounts::default();
        for &msat in denominations_msat {
            let amount = Amount::from_msats(msat);
            tiered_counts.inc(amount, 1);
        }

        let operation_id = mint
            .spend_notes_with_exact_denominations(tiered_counts, ())
            .await?;

        let outcome = mint
            .subscribe_spend_notes_with_exact_denominations(operation_id)
            .await?;

        let notes = match outcome
            .await_outcome()
            .await
            .ok_or_else(|| anyhow::anyhow!("No outcome reached for spend-exact"))?
        {
            SpendExactState::Success(notes) => notes,
            SpendExactState::Failed(e) => {
                anyhow::bail!("Spend-exact failed: {e}");
            }
            SpendExactState::Reissuing => {
                anyhow::bail!("Unexpected reissuing final state");
            }
        };

        // Split into individual OOBNotes strings
        let mut result = Vec::new();
        let federation_id_prefix = self.invite_code.federation_id().to_prefix();

        for (amount, note) in notes.iter_items() {
            let mut single = TieredMulti::default();
            single.push(amount, *note);
            let oob = if include_invite {
                OOBNotes::new_with_invite(single, &self.invite_code)
            } else {
                OOBNotes::new(federation_id_prefix, single)
            };
            result.push(oob.to_string());
        }

        Ok(result)
    }

    async fn ensure_client(&self) -> anyhow::Result<Rc<ClientHandle>> {
        if let Some(client) = self.client.borrow().clone() {
            return Ok(client);
        }

        let builder = Self::client_builder().await?;
        let federation_id = self.invite_code.federation_id();
        let root_secret = RootSecret::StandardDoubleDerive(derive_federation_secret(
            &self.mnemonic,
            &federation_id,
        ));

        let client = if Client::is_initialized(&self.database).await {
            info!("Opening existing fedimint client for issuance");
            Rc::new(
                builder
                    .open(
                        self.connectors.clone(),
                        self.database.clone(),
                        root_secret,
                    )
                    .await?,
            )
        } else {
            info!("Joining federation for new issuance");
            let preview = builder
                .preview(self.connectors.clone(), &self.invite_code)
                .await?;
            Rc::new(preview.join(self.database.clone(), root_secret).await?)
        };

        self.client.borrow_mut().replace(client.clone());
        Ok(client)
    }

    async fn client_builder() -> anyhow::Result<fedimint_client::ClientBuilder> {
        let mut builder = Client::builder().await?;
        builder.with_module(MintClientInit);
        builder.with_module(LightningClientInit::default());
        builder.with_module(WalletClientInit(None));
        builder.with_module(MetaClientInit);
        let meta_source: MetaModuleMetaSourceWithFallback = Default::default();
        builder.with_meta_service(MetaService::new(meta_source));
        Ok(builder)
    }
}

fn derive_federation_secret(
    mnemonic: &Mnemonic,
    federation_id: &fedimint_core::config::FederationId,
) -> DerivableSecret {
    let global_root_secret = Bip39RootSecretStrategy::<12>::to_root_secret(mnemonic);
    let multi_federation_root_secret = global_root_secret.child_key(ChildId(0));
    let federation_root_secret = multi_federation_root_secret.federation_key(federation_id);
    let federation_wallet_root_secret = federation_root_secret.child_key(ChildId(0));
    federation_wallet_root_secret.child_key(ChildId(0))
}

fn parse_operation_id(s: &str) -> anyhow::Result<OperationId> {
    OperationId::from_str(s)
}
