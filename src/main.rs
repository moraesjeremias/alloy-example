use std::{env, ops::Mul};

use alloy::{
    network::TransactionBuilder,
    primitives::{Address, U256, utils::Unit},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use anyhow::{Ok, Result};
use dotenv::dotenv;
use tracing::info;

use crate::logs::initialize_logger;

mod logs;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    initialize_logger()?;

    send_tx().await?;

    return Ok(());
}

async fn send_tx() -> Result<()> {
    let from_signer: PrivateKeySigner = env::var("FROM_ADDRESS_PRIVATE_KEY").unwrap().parse()?;
    let rpc_url: String = env::var("RPC_URL").unwrap_or("https://sepolia.infura.io".to_string());
    let provider = ProviderBuilder::new()
        .wallet(from_signer)
        .connect(rpc_url.as_str())
        .await?;

    let to_address: Address = env::var("TO_ADDRESS").unwrap_or("".to_string()).parse()?;
    let value = Unit::ETHER.wei().saturating_mul(U256::from(0));

    let tx_request = TransactionRequest::default()
        .with_to(to_address)
        .with_value(value);

    let pending_tx = provider.send_transaction(tx_request).await?;

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
