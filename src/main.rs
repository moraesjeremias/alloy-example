use std::{env, ops::Mul, time::Duration};

use alloy::{
    network::{Ethereum, EthereumWallet, TransactionBuilder},
    primitives::{Address, U256, utils::Unit},
    providers::{
        Identity, Provider, ProviderBuilder, RootProvider,
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            WalletFiller,
        },
    },
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use anyhow::{Ok, Result};
use dotenv::dotenv;
use tokio::time::{Instant, sleep_until};
use tracing::info;

use crate::logs::initialize_logger;

mod logs;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    initialize_logger()?;

    handle_proc().await?;

    return Ok(());
}

async fn handle_proc() -> Result<()> {
    let from_signer: PrivateKeySigner = env::var("FROM_ADDRESS_PRIVATE_KEY").unwrap().parse()?;
    let rpc_url: String = env::var("RPC_URL").unwrap_or("https://sepolia.infura.io".to_string());
    let provider = ProviderBuilder::new()
        .wallet(from_signer)
        .connect(rpc_url.as_str())
        .await?;

    let max_gas_fee_threshold: u128 = env::var("MAX_GAS_FEE_THRESHOLD")
        .unwrap_or("1000000000".to_string())
        .parse()?;

    let to_address: Address = env::var("TO_ADDRESS").unwrap_or("".to_string()).parse()?;
    let value = Unit::ETHER.wei().saturating_mul(U256::from(0));

    let tx_request = TransactionRequest::default()
        .with_to(to_address)
        .with_value(value);

    let mut effective_gas_estimate = estimate_effective_gas(&provider, &tx_request).await;

    for _attempt in 1..5 {
        if effective_gas_estimate > max_gas_fee_threshold {
            info!("Gas fee too high");
            sleep_until(Instant::now() + Duration::from_secs(12)).await;
            effective_gas_estimate = estimate_effective_gas(&provider, &tx_request).await;
        } else {
            send_tx(&provider, &tx_request).await?;
            break;
        }
    }

    return Ok(());
}

async fn send_tx<P>(provider: &P, tx_request: &TransactionRequest) -> Result<()>
where
    P: Provider<Ethereum> + Clone,
{
    let pending_tx = provider
        .clone()
        .send_transaction(tx_request.clone())
        .await?;

    info!("Pending tx {}", pending_tx.tx_hash());

    let receipt = pending_tx.get_receipt().await?;
    let gas_used: u128 = receipt.gas_used as u128;

    let effective_gas = gas_used.mul(receipt.effective_gas_price);

    info!(
        message = "Transaction gas fee used",
        gas_fee = effective_gas,
        tx_hash = %receipt.transaction_hash,
    );

    return Ok(());
}

async fn estimate_effective_gas<P>(provider: &P, tx_request: &TransactionRequest) -> u128
where
    P: Provider<Ethereum>,
{
    let gas_estimate = provider.estimate_gas(tx_request.clone()).await.unwrap() as u128;

    let max_fee_per_gas = provider
        .estimate_eip1559_fees()
        .await
        .unwrap()
        .max_fee_per_gas;
    let effective_gas_estimate = gas_estimate.mul(max_fee_per_gas);
    info!(
        message = "Gas estimate",
        gas_estimate = gas_estimate,
        effective_gas_estimate = effective_gas_estimate,
        max_fee_per_gas = max_fee_per_gas
    );

    return effective_gas_estimate;
}
