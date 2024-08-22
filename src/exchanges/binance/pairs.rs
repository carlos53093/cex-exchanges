use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{exchanges::normalized::types::NormalizedTradingPair, normalized::types::NormalizedTradingType, CexExchange};

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd)]
pub struct BinanceTradingPair(pub(crate) String);

impl BinanceTradingPair {
    pub fn new_checked(s: &str) -> eyre::Result<Self> {
        s.to_string().try_into()
    }

    pub fn is_valid(s: &str) -> bool {
        !s.contains('-') && !s.contains('_') && !s.contains('/')
    }

    pub fn normalize(&self) -> NormalizedTradingPair {
        NormalizedTradingPair::new_no_base_quote(CexExchange::Binance, &self.0)
    }

    pub fn normalize_with(&self, base: &str, quote: &str) -> NormalizedTradingPair {
        NormalizedTradingPair::new_base_quote(CexExchange::Binance, base, quote, None, None)
    }
}

impl Display for BinanceTradingPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for BinanceTradingPair {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BinanceTradingPair {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;

        Ok(BinanceTradingPair(s))
    }
}

impl TryFrom<NormalizedTradingPair> for BinanceTradingPair {
    type Error = eyre::Report;

    fn try_from(value: NormalizedTradingPair) -> Result<Self, Self::Error> {
        if let Some((base, quote)) = value.base_quote() {
            return Ok(BinanceTradingPair(format!("{}{}", base, quote)))
        }

        if let (Some(raw_pair), delim) = (value.pair(), value.delimiter()) {
            if let Ok(v) = Self::new_checked(raw_pair) {
                return Ok(v)
            }

            if let Some(d) = delim {
                let mut split = raw_pair.split(d);
                return Ok(BinanceTradingPair(format!("{}{}", split.next().unwrap().to_uppercase(), split.next().unwrap().to_uppercase())));
            }

            let new_str = raw_pair.replace(['_', '-', '/'], "");
            if let Ok(this) = Self::new_checked(&new_str) {
                return Ok(this)
            }

            return Err(eyre::ErrReport::msg(format!("INVALID Binance trading pair '{raw_pair}'")))
        }

        Err(eyre::ErrReport::msg(format!("INVALID Binance trading pair '{:?}'", value)))
    }
}

impl TryFrom<&str> for BinanceTradingPair {
    type Error = eyre::Report;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if Self::is_valid(value) {
            Ok(BinanceTradingPair(value.to_uppercase()))
        } else {
            Err(eyre::ErrReport::msg(format!("INVALID Binance trading pair '{value}' contains a '-', '_', or '/'")))
        }
    }
}

impl TryFrom<String> for BinanceTradingPair {
    type Error = eyre::Report;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize, PartialEq, PartialOrd)]
#[serde(rename_all = "UPPERCASE")]
pub enum BinanceTradingType {
    Spot,
    Perpetual,
    Margin,
    Futures,
    Option,
    #[default]
    Other
}
impl From<BinanceTradingType> for NormalizedTradingType {
    fn from(val: BinanceTradingType) -> Self {
        match val {
            BinanceTradingType::Perpetual => NormalizedTradingType::Perpetual,
            BinanceTradingType::Margin => NormalizedTradingType::Margin,
            BinanceTradingType::Spot => NormalizedTradingType::Spot,
            BinanceTradingType::Option => NormalizedTradingType::Option,
            BinanceTradingType::Futures => NormalizedTradingType::Futures,
            BinanceTradingType::Other => NormalizedTradingType::Other
        }
    }
}

impl FromStr for BinanceTradingType {
    type Err = eyre::Report;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let s = value.to_lowercase();

        match s.as_str() {
            "spot" => Ok(BinanceTradingType::Spot),
            "perpetual" | "perp" | "swap" | "linear" | "inverse" => Ok(BinanceTradingType::Perpetual),
            "futures" => Ok(BinanceTradingType::Futures),
            "margin" => Ok(BinanceTradingType::Margin),
            "option" => Ok(BinanceTradingType::Option),
            _ => Ok(BinanceTradingType::Other)
        }
    }
}
