//! Operations for federations running the v2 mint (`mintv2`) and Lightning
//! (`lnv2`) modules. [`super::WalletRuntimeCore`] dispatches here when the
//! federation's primary mint is `mintv2` or it has no v1 Lightning module.

use std::str::FromStr;

use fedimint_client::ClientHandle;
use fedimint_core::base32::{self, FEDIMINT_PREFIX};
use fedimint_core::module::{AmountUnit, CommonModuleInit};
use fedimint_core::Amount;
use fedimint_lnv2_client::{
    FinalReceiveOperationState, FinalSendOperationState, LightningClientModule,
    LightningOperationMeta,
};
use fedimint_lnv2_common::config::LightningClientConfig;
use fedimint_lnv2_common::gateway_api::PaymentFee;
use fedimint_lnv2_common::{Bolt11InvoiceDescription, LightningCommonInit, LightningInvoice};
use fedimint_mintv2_client::{ECash, MintClientModule, MintOperationMeta};
use fedimint_mintv2_common::config::MintClientConfig;
use fedimint_mintv2_common::{Denomination, MintCommonInit};
use serde_json::Value;

use super::parse_operation_id;

/// Matches the 24h default expiry of v1 invoices; also the LNv2 maximum.
const INVOICE_EXPIRY_SECS: u32 = 60 * 60 * 24;

/// Whether the client's primary (balance-holding) module is the v2 mint.
pub(super) fn uses_mintv2(client: &ClientHandle) -> bool {
    let Some((primary, _)) = client.primary_module_for_unit(AmountUnit::BITCOIN) else {
        return false;
    };
    client.get_first_instance(&MintCommonInit::KIND) == Some(primary)
}

/// Whether Lightning should go through LNv2, i.e. the federation has no v1
/// Lightning module.
pub(super) fn uses_lnv2(client: &ClientHandle) -> bool {
    client
        .get_first_module::<fedimint_ln_client::LightningClientModule>()
        .is_err()
        && client.get_first_module::<LightningClientModule>().is_ok()
}

/// See [`super::WalletRuntimeCore::get_ecash_fee_budget`].
pub(super) async fn ecash_fee_budget(client: &ClientHandle) -> anyhow::Result<u64> {
    let config = client.config().await;

    let mint_instance = client
        .get_first_instance(&MintCommonInit::KIND)
        .ok_or_else(|| anyhow::anyhow!("No mintv2 module"))?;
    let mint_cfg: &MintClientConfig = config.modules[&mint_instance].cast()?;

    let mint = client.get_first_module::<MintClientModule>()?.inner();
    let note_fee_msat: u64 = mint
        .get_count_by_denomination()
        .await
        .into_iter()
        .map(|(denomination, count)| {
            mint_cfg.fee_consensus.fee(denomination.amount()).msats * count
        })
        .sum();

    // LNv2 additionally charges a federation fee on the outgoing contract
    let ln_fee_msat = match client.get_first_instance(&LightningCommonInit::KIND) {
        Some(ln_instance) => {
            let ln_cfg: &LightningClientConfig = config.modules[&ln_instance].cast()?;
            ln_cfg
                .fee_consensus
                .fee(client.get_balance_for_btc().await?)
                .msats
        }
        None => 0,
    };

    // Budget 1.2x to cover change outputs and rounding
    Ok((note_fee_msat + ln_fee_msat) * 6 / 5)
}

/// See [`super::WalletRuntimeCore::get_gateway_fees`].
pub(super) async fn gateway_fees(client: &ClientHandle) -> anyhow::Result<(u32, u32)> {
    let ln = client.get_first_module::<LightningClientModule>()?.inner();
    let (_, routing_info) = ln.select_gateway(None).await?;
    // The default send fee also covers routing over Lightning, not just a
    // direct swap with the gateway
    let fee = routing_info.send_fee_default;
    Ok((
        u32::try_from(fee.base.msats)?,
        u32::try_from(fee.parts_per_million)?,
    ))
}

pub(super) async fn create_invoice(
    client: &ClientHandle,
    amount_msat: u64,
    description: &str,
) -> anyhow::Result<(String, String)> {
    let ln = client.get_first_module::<LightningClientModule>()?.inner();
    let (gateway, routing_info) = ln.select_gateway(None).await?;

    // Unlike v1, LNv2 gateways deduct their fee from the incoming payment, so
    // the invoice has to be larger for `amount_msat` to reach the wallet.
    let invoice_amount_msat = add_receive_fee(amount_msat, &routing_info.receive_fee);

    let (invoice, operation_id) = ln
        .receive(
            Amount::from_msats(invoice_amount_msat),
            INVOICE_EXPIRY_SECS,
            Bolt11InvoiceDescription::Direct(description.to_owned()),
            Some(gateway),
            Value::Null,
        )
        .await?;
    Ok((operation_id.fmt_full().to_string(), invoice.to_string()))
}

/// Invoice amount that leaves at least `amount_msat` (at most one msat more)
/// after the gateway's receive fee is subtracted.
fn add_receive_fee(amount_msat: u64, fee: &PaymentFee) -> u64 {
    // fee(x) = base + x * ppm / 1e6, so solve x - fee(x) >= amount for x
    let remaining_millionths = 1_000_000u64.saturating_sub(fee.parts_per_million).max(1);
    let mut invoice_msat = (u128::from(amount_msat + fee.base.msats) * 1_000_000)
        .div_ceil(u128::from(remaining_millionths)) as u64;
    // Correct for the fee formula rounding down
    while fee.subtract_from(invoice_msat).msats < amount_msat {
        invoice_msat += 1;
    }
    invoice_msat
}

pub(super) async fn pay_bolt11(client: &ClientHandle, invoice_str: &str) -> anyhow::Result<()> {
    let ln = client.get_first_module::<LightningClientModule>()?.inner();
    let invoice = lightning_invoice::Bolt11Invoice::from_str(invoice_str)
        .map_err(|e| anyhow::anyhow!("Invalid invoice: {e}"))?;

    let operation_id = ln.send(invoice, None, Value::Null).await?;

    match ln.await_final_send_operation_state(operation_id).await? {
        FinalSendOperationState::Success(_) => Ok(()),
        FinalSendOperationState::Refunded => anyhow::bail!("Payment refunded"),
        FinalSendOperationState::Failure => anyhow::bail!("Payment failed"),
    }
}

pub(super) async fn reissue_notes(client: &ClientHandle, notes_str: &str) -> anyhow::Result<()> {
    let mint = client.get_first_module::<MintClientModule>()?.inner();

    let ecash: ECash = base32::decode_prefixed(FEDIMINT_PREFIX, notes_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse ECash: {e}"))?;

    let operation_id = mint.receive(ecash, Value::Null).await?;

    match mint
        .await_final_receive_operation_state(operation_id)
        .await?
    {
        fedimint_mintv2_client::FinalReceiveOperationState::Success => Ok(()),
        fedimint_mintv2_client::FinalReceiveOperationState::Rejected => {
            anyhow::bail!("Reissue rejected, notes were already spent")
        }
    }
}

pub(super) async fn spend_exact(
    client: &ClientHandle,
    denominations_msat: &[u64],
) -> anyhow::Result<String> {
    let mint = client.get_first_module::<MintClientModule>()?.inner();

    let denominations = denominations_msat
        .iter()
        .map(|&msat| {
            anyhow::ensure!(msat.is_power_of_two(), "Invalid denomination: {msat} msat");
            Ok(Denomination(msat.trailing_zeros() as u8))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let (_, ecash) = mint
        .send_denominations(denominations, Value::Null, true)
        .await?;

    Ok(base32::encode_prefixed(FEDIMINT_PREFIX, &ecash))
}

pub(super) async fn find_ln_receive(
    client: &ClientHandle,
) -> anyhow::Result<Option<(String, String)>> {
    let ops = client
        .operation_log()
        .paginate_operations_rev(100, None)
        .await;

    for (key, entry) in &ops {
        if entry.operation_module_kind() != LightningCommonInit::KIND.as_str() {
            continue;
        }
        let Ok(LightningOperationMeta::Receive(meta)) = entry.try_meta::<LightningOperationMeta>()
        else {
            continue;
        };
        let LightningInvoice::Bolt11(invoice) = meta.invoice;
        return Ok(Some((
            key.operation_id.fmt_full().to_string(),
            invoice.to_string(),
        )));
    }
    Ok(None)
}

pub(super) async fn wait_for_receive(
    client: &ClientHandle,
    operation_id_str: &str,
) -> anyhow::Result<()> {
    let op_id = parse_operation_id(operation_id_str)?;
    let ln = client.get_first_module::<LightningClientModule>()?.inner();
    match ln.await_final_receive_operation_state(op_id).await? {
        FinalReceiveOperationState::Claimed => Ok(()),
        FinalReceiveOperationState::Expired => anyhow::bail!("LN receive expired"),
        FinalReceiveOperationState::Failure => anyhow::bail!("LN receive failed"),
    }
}

/// Every send in an issuance wallet is a paper note, and the operation log
/// keeps the encoded ecash of each.
pub(super) async fn recover_issued_notes(client: &ClientHandle) -> anyhow::Result<Vec<String>> {
    let ops = client
        .operation_log()
        .paginate_operations_rev(10000, None)
        .await;

    Ok(ops
        .iter()
        .filter(|(_, entry)| entry.operation_module_kind() == MintCommonInit::KIND.as_str())
        .filter_map(|(_, entry)| match entry.try_meta::<MintOperationMeta>() {
            Ok(MintOperationMeta::Send { ecash, .. }) => Some(ecash),
            _ => None,
        })
        .collect())
}
