use serde::{Deserialize, Serialize};

use super::Blockchain;
use crate::exchanges::CexExchange;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct NormalizedCurrency {
    pub exchange:     CexExchange,
    pub symbol:       String,
    pub name:         String,
    pub display_name: Option<String>,
    pub status:       String,
    pub blockchains:  Vec<BlockchainCurrency>
}

impl NormalizedCurrency {
    /// takes a vector of [NormalizedCurrency] and deduplicates itself by
    /// combining wrapped and unwrapped assets
    pub(crate) fn handle_unwrapped(mut normalized: Vec<Self>) -> Vec<Self> {
        let unwrapped = normalized
            .iter()
            .filter(|&curr| !curr.blockchains.iter().any(|b| b.is_wrapped))
            .cloned()
            .collect::<Vec<_>>();

        let mut to_remove = Vec::new();

        let wrapped = normalized
            .iter()
            .filter_map(|curr| {
                if curr.blockchains.iter().any(|b| b.is_wrapped) {
                    let (new, removed) = curr.combine_wrapped_assets(&unwrapped);

                    removed.map(|r| {
                        to_remove.push(r);
                        to_remove.push(curr.clone());

                        new
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        normalized.retain(|val| !to_remove.contains(val));
        normalized.extend(wrapped);

        normalized
    }

    /// takes a wrapped currency and a vec of unwrapped currencies
    /// returns either:
    ///     - an unwrapped currency with the `blockchains.wrapped_currency`
    ///       field updated + the old value
    ///     - itself if no associated unwrapped currencies were found + none
    fn combine_wrapped_assets(&self, unwrapped_currencies: &[NormalizedCurrency]) -> (Self, Option<Self>) {
        let un = self.name.to_lowercase().replace("wrapped", "");
        let unwrapped_name = un.trim();
        let unwrapped_symbol = self.symbol[1..].to_string();

        unwrapped_currencies
            .iter()
            .find(|curr| curr.name.to_lowercase() == unwrapped_name && curr.symbol == unwrapped_symbol)
            .cloned()
            .map(|mut curr| {
                let to_remove = curr.clone();
                let mut blockchains = self.blockchains.clone();
                blockchains.iter_mut().for_each(|b| {
                    if b.is_wrapped {
                        b.wrapped_currency = Some(WrappedCurrency { symbol: self.symbol.clone(), name: self.name.clone() })
                    }
                });

                curr.blockchains.extend(blockchains);

                (curr.clone(), Some(to_remove))
            })
            .unwrap_or((self.clone(), None))
    }

    /// checks if the currency is found in a certain blockchain
    pub fn has_blockchain(&self, chain: &Blockchain) -> bool {
        self.blockchains.iter().any(|c| &c.blockchain == chain)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct BlockchainCurrency {
    pub blockchain:       Blockchain,
    pub address:          Option<String>,
    /// (true & wrapped_currency) == None -> tbd
    pub is_wrapped:       bool,
    pub wrapped_currency: Option<WrappedCurrency>
}

impl BlockchainCurrency {
    pub fn wrapped(&mut self, is_wrapped: bool) {
        self.is_wrapped = is_wrapped;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct WrappedCurrency {
    pub symbol: String,
    pub name:   String
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "non-us")]
    const TEST_EXCHANGE: CexExchange = CexExchange::Binance;
    #[cfg(not(feature = "non-us"))]
    const TEST_EXCHANGE: CexExchange = CexExchange::Coinbase;

    #[test]
    fn test_combine_wrapped_assets_basic() {
        let wrapped = NormalizedCurrency {
            exchange:     TEST_EXCHANGE,
            symbol:       "WETH".to_string(),
            name:         "Wrapped Ethereum".to_string(),
            display_name: None,
            status:       String::new(),
            blockchains:  vec![BlockchainCurrency {
                blockchain:       Blockchain::Ethereum,
                address:          Some("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string()),
                is_wrapped:       true,
                wrapped_currency: None
            }]
        };

        let unwrapped = NormalizedCurrency {
            exchange:     TEST_EXCHANGE,
            symbol:       "ETH".to_string(),
            name:         "Ethereum".to_string(),
            display_name: None,
            status:       String::new(),
            blockchains:  Vec::new()
        };

        let to_remove = unwrapped.clone();
        let combined = wrapped.combine_wrapped_assets(&vec![unwrapped]);

        let expected = NormalizedCurrency {
            exchange:     TEST_EXCHANGE,
            symbol:       "ETH".to_string(),
            name:         "Ethereum".to_string(),
            display_name: None,
            status:       String::new(),
            blockchains:  vec![BlockchainCurrency {
                blockchain:       Blockchain::Ethereum,
                address:          Some("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string()),
                is_wrapped:       true,
                wrapped_currency: Some(WrappedCurrency { symbol: "WETH".to_string(), name: "Wrapped Ethereum".to_string() })
            }]
        };

        assert_eq!(combined, (expected, Some(to_remove)))
    }

    #[test]
    fn test_combine_wrapped_assets_2chain_address() {
        let wrapped = NormalizedCurrency {
            exchange:     TEST_EXCHANGE,
            symbol:       "WETH".to_string(),
            name:         "Wrapped Ethereum".to_string(),
            display_name: None,
            status:       String::new(),
            blockchains:  vec![BlockchainCurrency {
                blockchain:       Blockchain::Ethereum,
                address:          Some("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string()),
                is_wrapped:       true,
                wrapped_currency: None
            }]
        };

        let unwrapped = NormalizedCurrency {
            exchange:     TEST_EXCHANGE,
            symbol:       "ETH".to_string(),
            name:         "Ethereum".to_string(),
            display_name: None,
            status:       String::new(),
            blockchains:  vec![BlockchainCurrency {
                blockchain:       Blockchain::Ethereum,
                address:          Some("0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD".to_string()),
                is_wrapped:       false,
                wrapped_currency: None
            }]
        };

        let to_remove = unwrapped.clone();
        let combined = wrapped.combine_wrapped_assets(&vec![unwrapped]);

        let expected = NormalizedCurrency {
            exchange:     TEST_EXCHANGE,
            symbol:       "ETH".to_string(),
            name:         "Ethereum".to_string(),
            display_name: None,
            status:       String::new(),
            blockchains:  vec![
                BlockchainCurrency {
                    blockchain:       Blockchain::Ethereum,
                    address:          Some("0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD".to_string()),
                    is_wrapped:       false,
                    wrapped_currency: None
                },
                BlockchainCurrency {
                    blockchain:       Blockchain::Ethereum,
                    address:          Some("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string()),
                    is_wrapped:       true,
                    wrapped_currency: Some(WrappedCurrency { symbol: "WETH".to_string(), name: "Wrapped Ethereum".to_string() })
                },
            ]
        };

        assert_eq!(combined, (expected, Some(to_remove)))
    }

    #[test]
    fn test_deduplicate_vec_self() {
        let wrapped = NormalizedCurrency {
            exchange:     TEST_EXCHANGE,
            symbol:       "WETH".to_string(),
            name:         "Wrapped Ethereum".to_string(),
            display_name: None,
            status:       String::new(),
            blockchains:  vec![BlockchainCurrency {
                blockchain:       Blockchain::Ethereum,
                address:          Some("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string()),
                is_wrapped:       true,
                wrapped_currency: None
            }]
        };

        let unwrapped = NormalizedCurrency {
            exchange:     TEST_EXCHANGE,
            symbol:       "ETH".to_string(),
            name:         "Ethereum".to_string(),
            display_name: None,
            status:       String::new(),
            blockchains:  vec![BlockchainCurrency {
                blockchain:       Blockchain::Ethereum,
                address:          Some("0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD".to_string()),
                is_wrapped:       false,
                wrapped_currency: None
            }]
        };

        let combined = NormalizedCurrency::handle_unwrapped(vec![wrapped.clone(), unwrapped.clone()]);

        let expected = NormalizedCurrency {
            exchange:     TEST_EXCHANGE,
            symbol:       "ETH".to_string(),
            name:         "Ethereum".to_string(),
            display_name: None,
            status:       String::new(),
            blockchains:  vec![
                BlockchainCurrency {
                    blockchain:       Blockchain::Ethereum,
                    address:          Some("0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD".to_string()),
                    is_wrapped:       false,
                    wrapped_currency: None
                },
                BlockchainCurrency {
                    blockchain:       Blockchain::Ethereum,
                    address:          Some("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string()),
                    is_wrapped:       true,
                    wrapped_currency: Some(WrappedCurrency { symbol: "WETH".to_string(), name: "Wrapped Ethereum".to_string() })
                },
            ]
        };

        assert_eq!(combined, vec![expected])
    }
}
