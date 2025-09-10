use std::{
    env,
    ops::{Div, Mul},
    time::Duration,
};

use alloy::{
    network::{Ethereum, TransactionBuilder},
    primitives::{Address, U256, utils::Unit},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use anyhow::{Ok, Result};
use dotenv::dotenv;
use tokio::time::{Instant, sleep_until};
use tracing::info;

use crate::{logs::initialize_logger, types::CoingekoApiResponse};

mod logs;
mod types;

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

    let max_gas_fee_threshold: f64 = env::var("MAX_GAS_FEE_THRESHOLD")
        .unwrap_or("0.01".to_string())
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

    let wei: f64 = Unit::ETHER.wei_const().to::<u64>() as f64;

    let gas_price: f64 = receipt.effective_gas_price as f64;
    let effective_gas_used = gas_price.mul(receipt.gas_used as f64).div(wei);

    let usd_fee = convert_gas_fee(effective_gas_used).await;

    info!(
        message = "Transaction gas fee used",
        gas_fee = effective_gas_used,
        usd_fee = usd_fee,
        tx_hash = %receipt.transaction_hash,
    );

    return Ok(());
}

async fn estimate_effective_gas<P>(provider: &P, tx_request: &TransactionRequest) -> f64
where
    P: Provider<Ethereum>,
{
    let wei = Unit::ETHER.wei_const().to::<u64>() as f64;
    let gas_estimate = provider.estimate_gas(tx_request.clone()).await.unwrap() as f64;

    let max_fee_per_gas = provider
        .estimate_eip1559_fees()
        .await
        .unwrap()
        .max_fee_per_gas as f64;
    let effective_gas_estimate = gas_estimate.mul(max_fee_per_gas).div(wei);
    let usd_estimate = convert_gas_fee(effective_gas_estimate).await;
    info!(
        message = "Gas estimate",
        gas_estimate = gas_estimate,
        effective_gas_estimate = effective_gas_estimate,
        max_fee_per_gas = max_fee_per_gas,
        usd_estimate = usd_estimate
    );

    return effective_gas_estimate;
}

pub async fn fetch_eth_to_usd_rate() -> CoingekoApiResponse {
    let coingeko_url =
        env::var("COINGEKO_URL").unwrap_or("https://api.coingecko.com/api".to_string());
    let coingeko_api_key = env::var("COINGEKO_API_KEY").expect("Missing COINGEKO_API_KEY env");
    let price_endpoint = format!(
        "{}/v3/simple/price?ids={}&vs_currencies={}&x_cg_api_key={}",
        coingeko_url, "ethereum", "usd", coingeko_api_key
    );
    let response = reqwest::get(price_endpoint).await.unwrap();

    response.json::<CoingekoApiResponse>().await.unwrap()
}

pub async fn convert_gas_fee(gas_fee: f64) -> f64 {
    let eth_to_usd_rate = fetch_eth_to_usd_rate().await;
    return gas_fee.mul(eth_to_usd_rate.from_asset.to_asset);
}
