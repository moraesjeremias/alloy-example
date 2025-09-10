use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CoingekoApiResponse {
    #[serde(alias = "ethereum")]
    pub from_asset: EthToUSDRate,
}

#[derive(Debug, Deserialize)]
pub struct EthToUSDRate {
    #[serde(alias = "usd")]
    pub to_asset: f64,
}
