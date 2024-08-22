use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::serde_as;
use tracing::warn;

use crate::{
    normalized::{
        rest_api::NormalizedRestApiDataTypes,
        types::{BlockchainCurrency, NormalizedCurrency}
    },
    CexExchange
};

#[serde_as]
#[derive(Debug, Clone, Serialize, PartialEq, PartialOrd)]
pub struct BinanceAllSymbols {
    pub symbols: Vec<BinanceSymbol>
}

impl BinanceAllSymbols {
    pub fn normalize(self) -> Vec<NormalizedCurrency> {
        let normalized = self
            .symbols
            .into_iter()
            .map(BinanceSymbol::normalize)
            .collect::<Vec<_>>();

        NormalizedCurrency::handle_unwrapped(normalized)
    }
}

impl PartialEq<NormalizedRestApiDataTypes> for BinanceAllSymbols {
    fn eq(&self, other: &NormalizedRestApiDataTypes) -> bool {
        match other {
            NormalizedRestApiDataTypes::AllCurrencies(other_currs) => {
                let this_currencies = self
                    .symbols
                    .iter()
                    .map(|sym| (&sym.name, &sym.symbol))
                    .collect::<HashSet<_>>();

                let others_currencies = other_currs.clone();
                let mut normalized_out = 0;

                others_currencies.iter().for_each(|curr| {
                    if curr.blockchains.iter().any(|blk| {
                        if let Some(blk_curr) = blk.wrapped_currency.as_ref() {
                            blk.is_wrapped && blk_curr.name.to_lowercase().contains("wrapped") && blk_curr.symbol.to_lowercase().starts_with('w')
                        } else {
                            false
                        }
                    }) {
                        normalized_out += 1;
                    }
                });

                self.symbols.len() == others_currencies.len() + normalized_out
                    && others_currencies
                        .iter()
                        .all(|curr| this_currencies.contains(&(&curr.name, &curr.symbol)))
            }
            _ => false
        }
    }
}

impl<'de> Deserialize<'de> for BinanceAllSymbols {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        let val = Value::deserialize(deserializer)?;

        let data = val
            .get("data")
            .ok_or(eyre::ErrReport::msg("Could not find 'data' field in Binance symbols with addresses request".to_string()))
            .map_err(serde::de::Error::custom)?
            .get("body")
            .ok_or(eyre::ErrReport::msg("Could not find 'body' field in Binance symbols with addresses request".to_string()))
            .map_err(serde::de::Error::custom)?
            .get("data")
            .ok_or(eyre::ErrReport::msg("Could not find nested 'data' field in Binance symbols with addresses request".to_string()))
            .map_err(serde::de::Error::custom)?
            .as_array()
            .ok_or(eyre::ErrReport::msg("Could not convert nested 'data' field in Binance symbols with addresses request to array".to_string()))
            .map_err(serde::de::Error::custom)?;

        let symbols = data
            .iter()
            .map(|v| serde_json::from_value(v.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(serde::de::Error::custom)?;

        Ok(BinanceAllSymbols { symbols })
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct BinanceSymbol {
    pub symbol: String,
    pub circulating_supply: f64,
    pub last_updated: DateTime<Utc>,
    pub total_supply: f64,
    pub tvl_ratio: Option<f64>,
    pub cmc_rank: u64,
    pub self_reported_circulating_supply: Option<f64>,
    pub platform: Option<BinanceSymbolPlatform>,
    pub tags: Vec<String>,
    pub date_added: DateTime<Utc>,
    pub quote: BinanceAllSymbolsQuote,
    pub num_market_pairs: u64,
    pub infinite_supply: bool,
    pub name: String,
    pub max_supply: Option<f64>,
    pub id: u64,
    pub self_reported_market_cap: Option<f64>,
    pub slug: String
}

impl BinanceSymbol {
    fn parse_blockchain(&self) -> Option<BlockchainCurrency> {
        if let Some(pl) = self.platform.as_ref() {
            let is_wrapped = self.name.to_lowercase().contains("wrapped") && self.symbol.to_lowercase().starts_with('w');
            Some(BlockchainCurrency {
                blockchain: pl.name.parse().unwrap(),
                address: Some(pl.token_address.clone()),
                is_wrapped,
                wrapped_currency: None
            })
        } else {
            None
        }
    }

    pub fn normalize(self) -> NormalizedCurrency {
        let blockchains = self.parse_blockchain().map(|p| vec![p]).unwrap_or_default();
        NormalizedCurrency {
            exchange: CexExchange::Binance,
            symbol: self.symbol,
            name: self.name,
            display_name: None,
            status: format!("last updated: {:?}", self.last_updated),
            blockchains
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct BinanceAllSymbolsQuote {
    #[serde(rename = "USD")]
    pub usd: BinanceSymbolQuoteUSD
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct BinanceSymbolQuoteUSD {
    pub fully_diluted_market_cap: f64,
    pub last_updated: DateTime<Utc>,
    pub market_cap_dominance: f64,
    pub tvl: Option<f64>,
    pub percent_change_30d: f64,
    pub percent_change_1h: f64,
    pub percent_change_24h: f64,
    pub market_cap: f64,
    pub volume_change_24h: f64,
    pub price: f64,
    pub percent_change_60d: f64,
    pub volume_24h: f64,
    pub percent_change_90d: f64,
    pub percent_change_7d: f64
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct BinanceSymbolPlatform {
    pub symbol:        String,
    pub name:          String,
    pub token_address: String,
    pub id:            u64,
    pub slug:          String
}

impl PartialEq<NormalizedCurrency> for BinanceSymbol {
    fn eq(&self, other: &NormalizedCurrency) -> bool {
        let blockchains = self.parse_blockchain().map(|p| vec![p]).unwrap_or_default();
        let equals = other.exchange == CexExchange::Binance
            && other.symbol == self.symbol
            && other.name == self.name
            && other.display_name.is_none()
            && other.status == format!("last updated: {:?}", self.last_updated)
            && other
                .blockchains
                .iter()
                .filter(|&blk| blk.wrapped_currency.is_none())
                .cloned()
                .collect::<Vec<_>>()
                == blockchains;

        if !equals {
            warn!(target: "cex-exchanges::binance", "binance currency: {:?}", self);
            warn!(target: "cex-exchanges::binance", "normalized currency: {:?}", other);
        }

        equals
    }
}
