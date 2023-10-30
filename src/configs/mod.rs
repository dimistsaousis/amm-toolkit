use ethers::providers::{Http, Provider};
use ethers::types::H160;
use serde_yaml;
use std::error::Error;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::{collections::HashMap, fs};

use crate::amm::uniswap_v2::factory::UniswapV2Factory;
use crate::amm::uniswap_v2::UniswapV2Pool;
use crate::errors::AMMError;

#[derive(Debug)]
pub enum ConfigError {
    EnvVarMissing(String),
    MiddlewareInitError(String),
    TokensLoadError(String),
    UniswapPairsLoadError(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigError::EnvVarMissing(var) => write!(f, "Missing env variable: {}", var),
            ConfigError::MiddlewareInitError(e) => {
                write!(f, "Middleware initialization error: {}", e)
            }
            ConfigError::TokensLoadError(e) => write!(f, "Tokens loading error: {}", e),
            ConfigError::UniswapPairsLoadError(e) => {
                write!(f, "Uniswap pairs loading error: {}", e)
            }
        }
    }
}

impl Error for ConfigError {}

pub struct Config {
    pub middleware: Arc<Provider<Http>>,
    pub tokens: HashMap<String, H160>,
    pub uniswap_v2_pairs: HashMap<String, HashMap<String, H160>>,
    pub uniswap_v2_factory: UniswapV2Factory,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let rpc_endpoint = std::env::var("NETWORK_RPC")
            .map_err(|_| ConfigError::EnvVarMissing("NETWORK_RPC".to_string()))?;

        let middleware = Arc::new(
            Provider::<Http>::try_from(rpc_endpoint)
                .map_err(|e| ConfigError::MiddlewareInitError(e.to_string()))?,
        );

        Ok(Config {
            middleware,
            tokens: Self::load_tokens(),
            uniswap_v2_pairs: Self::load_uniswap_v2_pairs(),
            uniswap_v2_factory: UniswapV2Factory::new(
                H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap(),
                10000835,
                300,
            ),
        })
    }

    pub async fn pool(
        &self,
        token_0: &str,
        token_1: &str,
    ) -> Result<UniswapV2Pool, AMMError<Provider<Http>>> {
        UniswapV2Pool::new_from_address(
            self.uniswap_v2_pairs[token_0][token_1],
            300,
            self.middleware.clone(),
        )
        .await
    }

    fn load_tokens() -> HashMap<String, H160> {
        let content = fs::read_to_string("src/configs/erc20_tokens.yaml").unwrap();
        let raw_map: HashMap<String, String> = serde_yaml::from_str(&content).unwrap();
        raw_map
            .into_iter()
            .map(|(key, value)| {
                let h160_value = H160::from_str(&value).expect("Invalid H160 format");
                (key, h160_value)
            })
            .collect()
    }

    fn load_uniswap_v2_pairs() -> HashMap<String, HashMap<String, H160>> {
        let content = fs::read_to_string("src/configs/uniswap_v2_pairs.yaml").unwrap();
        let mut raw_map: HashMap<String, HashMap<String, H160>> =
            serde_yaml::from_str(&content).unwrap();
        let mut additions: Vec<(String, String, H160)> = Vec::new();
        for (token1, inner) in &raw_map {
            for (token2, address) in inner {
                additions.push((token2.clone(), token1.clone(), address.clone()));
            }
        }
        for (token1, token2, address) in additions {
            raw_map.entry(token1).or_default().insert(token2, address);
        }
        raw_map
    }
}
