use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    vec,
};

use ethers::{
    abi::{ParamType, Token},
    providers::Middleware,
    types::{Bytes, H160, U256},
};
use futures::future;
use indicatif::ProgressBar;

use crate::errors::AMMError;

use super::UniswapV2Pool;

use ethers::prelude::abigen;

abigen!(
    IGetUniswapV2PoolDataBatchRequest,
    "src/contracts/GetUniswapV2PoolDataBatchRequest.json";
    IGetUniswapV2PairsBatchRequest,
    "src/contracts/GetUniswapV2PairsBatchRequest.json";
    GetWethValueInPoolBatchRequest,
    "src/contracts/GetWethValueInPoolBatchRequest.json";
);

pub async fn get_uniswap_v2_pool_data_batch_request_single<M: Middleware>(
    pool_address: H160,
    fee: u32,
    middleware: Arc<M>,
) -> Result<UniswapV2Pool, AMMError<M>> {
    let pools =
        get_uniswap_v2_pool_data_batch_request(&vec![pool_address], fee, middleware).await?;

    if let Some(pool) = pools.get(0) {
        Ok(pool.clone())
    } else {
        Err(AMMError::<M>::BatchRequestError(pool_address))
    }
}

struct TokenHelper;

impl TokenHelper {
    fn token_to_address(token: &Token, address: H160) -> H160 {
        token.to_owned().into_address().expect(&format!(
            "Expected addresses for token and address {:?}",
            address
        ))
    }

    fn token_to_u<U: TryFrom<u128>>(token: &Token, address: H160) -> U {
        let value = token
            .to_owned()
            .into_uint()
            .expect(&format!(
                "Expected integer for token decimals and address {:?}",
                address
            ))
            .as_u128();

        U::try_from(value).unwrap_or_else(&|_| {
            panic!(
                "Failed to convert value {} to the desired unsigned integer type for address {:?}",
                value, address
            )
        })
    }

    fn token_to_uniswap_pool(token: &Token, address: H160, fee: u32) -> UniswapV2Pool {
        let tup = &token.clone().into_tuple().unwrap();
        UniswapV2Pool {
            token_a: TokenHelper::token_to_address(&tup[0], address),
            token_a_decimals: TokenHelper::token_to_u::<u8>(&tup[1], address),
            token_b: TokenHelper::token_to_address(&tup[2], address),
            token_b_decimals: TokenHelper::token_to_u::<u8>(&tup[3], address),
            reserve_0: TokenHelper::token_to_u::<u128>(&tup[4], address),
            reserve_1: TokenHelper::token_to_u::<u128>(&tup[5], address),
            address,
            fee,
        }
    }
}

pub async fn get_uniswap_v2_pool_data_batch_request<M: Middleware>(
    pair_addresses: &Vec<H160>,
    fee: u32,
    middleware: Arc<M>,
) -> Result<Vec<UniswapV2Pool>, AMMError<M>> {
    let target_addresses: Vec<Token> = pair_addresses
        .iter()
        .map(|&address| Token::Address(address))
        .collect();
    let constructor_args = Token::Tuple(vec![Token::Array(target_addresses)]);
    let deployer = IGetUniswapV2PoolDataBatchRequest::deploy(middleware.clone(), constructor_args)?;
    let return_data: Bytes = deployer.call_raw().await?;
    let return_data_tokens = ethers::abi::decode(
        &[ParamType::Array(Box::new(ParamType::Tuple(vec![
            ParamType::Address,   // token a
            ParamType::Uint(8),   // token a decimals
            ParamType::Address,   // token b
            ParamType::Uint(8),   // token b decimals
            ParamType::Uint(112), // reserve 0
            ParamType::Uint(112), // reserve 1
        ])))],
        &return_data,
    )?;

    let err = AMMError::<M>::BatchRequestError;

    let mut pools = vec![];
    return_data_tokens
        .into_iter()
        .next()
        .ok_or(err(H160::zero()))?
        .into_array()
        .ok_or(err(H160::zero()))?
        .into_iter()
        .enumerate()
        .for_each(|(idx, token)| {
            pools.push(TokenHelper::token_to_uniswap_pool(
                &token,
                pair_addresses[idx],
                fee,
            ));
        });

    Ok(pools)
}

pub async fn get_uniswap_v2_pairs_batch_request<M: Middleware>(
    factory_address: H160,
    from: U256,
    to: U256,
    middleware: Arc<M>,
) -> Result<Vec<H160>, AMMError<M>> {
    let mut pairs = vec![];
    let constructor_args = Token::Tuple(vec![
        Token::Uint(from),
        Token::Uint(to),
        Token::Address(factory_address),
    ]);
    let deployer = IGetUniswapV2PairsBatchRequest::deploy(middleware.clone(), constructor_args)?;
    let return_data: Bytes = deployer.call_raw().await?;
    let return_data_tokens = ethers::abi::decode(
        &[ParamType::Array(Box::new(ParamType::Address))],
        &return_data,
    )?;

    return_data_tokens
        .into_iter()
        .next()
        .ok_or(AMMError::<M>::BatchRequestError(factory_address))?
        .into_array()
        .ok_or(AMMError::<M>::BatchRequestError(factory_address))?
        .into_iter()
        .for_each({
            |token| match token.into_address() {
                Some(addr) => {
                    if !addr.is_zero() {
                        pairs.push(addr);
                    }
                }
                None => (),
            }
        });

    Ok(pairs)
}

async fn get_weth_value_in_pool_batch_request<M: Middleware>(
    addresses: Vec<H160>,
    weth_address: H160,
    factory_address: H160,
    middleware: Arc<M>,
    progress_bar: Option<Arc<Mutex<ProgressBar>>>,
) -> Result<HashMap<H160, U256>, AMMError<M>> {
    let addresses_token: Vec<Token> = addresses
        .iter()
        .map(|&address| Token::Address(address))
        .collect();
    let constructor_args = Token::Tuple(vec![
        Token::Array(addresses_token),
        Token::Address(weth_address),
        Token::Address(factory_address),
    ]);
    let deployer = GetWethValueInPoolBatchRequest::deploy(middleware.clone(), constructor_args)?;
    let return_data: Bytes = deployer
        .call_raw()
        .await
        .map_err(|_err| AMMError::OutOfGasError(addresses.clone()))?;
    let return_data_tokens = ethers::abi::decode(
        &[ParamType::Array(Box::new(ParamType::Uint(256)))],
        &return_data,
    )?;

    let mut weth_values_in_pools: HashMap<H160, U256> = HashMap::new();

    for token_array in return_data_tokens {
        if let Some(arr) = token_array.into_array() {
            for (idx, token) in arr.into_iter().enumerate() {
                if let Some(weth_value_in_pool) = token.into_uint() {
                    weth_values_in_pools.insert(addresses[idx], weth_value_in_pool);
                }
            }
        }
    }

    if let Some(progress_bar) = progress_bar {
        progress_bar.lock().unwrap().inc((addresses.len()) as u64);
    }
    Ok(weth_values_in_pools)
}

pub async fn get_weth_value_in_pools<M: Middleware>(
    addresses: Vec<H160>,
    weth_address: H160,
    factory_address: H160,
    middleware: Arc<M>,
    step: Option<usize>,
) -> Result<HashMap<H160, U256>, AMMError<M>> {
    let step = match step {
        Some(step) => step,
        None => 100,
    };
    println!(
        "Getting weth equivalent value of {} uniswap pools",
        addresses.len()
    );
    let pb = ProgressBar::new(addresses.len() as u64);
    let shared_pb = Arc::new(Mutex::new(pb));
    let mut futures: Vec<_> = vec![];
    for i in (0..addresses.len()).step_by(step) {
        futures.push(get_weth_value_in_pool_batch_request(
            addresses[i..(i + step).min(addresses.len())].to_vec(),
            weth_address,
            factory_address,
            middleware.clone(),
            Some(shared_pb.clone()),
        ));
    }
    let results: Vec<std::result::Result<HashMap<H160, ethers::types::U256>, AMMError<M>>> =
        future::join_all(futures).await;
    let mut weth_values_in_pools: HashMap<H160, U256> = HashMap::new();
    let mut failed_addresses: Vec<H160> = vec![];
    for result in results {
        match result {
            Ok(mut weth_values_in_pools_batch) => {
                weth_values_in_pools.extend(weth_values_in_pools_batch.drain())
            }
            Err(AMMError::OutOfGasError(failed_batch)) => {
                failed_addresses.extend(failed_batch);
            }
            Err(err) => return Err(err),
        }
    }

    let mut futures: Vec<_> = vec![];
    for address in failed_addresses {
        futures.push(get_weth_value_in_pool_batch_request(
            vec![address],
            weth_address,
            factory_address,
            middleware.clone(),
            None,
        ));
    }
    let results: Vec<std::result::Result<HashMap<H160, ethers::types::U256>, AMMError<M>>> =
        future::join_all(futures).await;
    for result in results {
        match result {
            Ok(mut weth_values_in_pools_batch) => {
                weth_values_in_pools.extend(weth_values_in_pools_batch.drain())
            }
            Err(err) => return Err(err),
        }
    }

    Ok(weth_values_in_pools)
}

#[cfg(test)]
mod tests {
    use super::*;

    use ethers::providers::{Http, Provider};
    use ethers::types::H160;
    use std::str::FromStr;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_get_uniswap_v2_pool_data_batch_request_single() {
        dotenv::dotenv().ok();
        let rpc_endpoint = std::env::var("NETWORK_RPC").expect("Missing NETWORK_RPC env variable");
        let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint).unwrap());
        let uniswap_v2_usdc_weth_pair_address =
            H160::from_str("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc").unwrap();

        let result = get_uniswap_v2_pool_data_batch_request_single(
            uniswap_v2_usdc_weth_pair_address,
            300,
            middleware.clone(),
        )
        .await;

        match result {
            Ok(pool) => {
                assert_eq!(pool.address, uniswap_v2_usdc_weth_pair_address);
                assert_eq!(
                    pool.token_a,
                    H160::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap()
                );
                assert_eq!(pool.token_a_decimals, 6);
                assert_eq!(
                    pool.token_b,
                    H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap()
                );
                assert_eq!(pool.token_b_decimals, 18);
                assert!(pool.reserve_0 > 0);
                assert!(pool.reserve_1 > 0);
                assert!(pool.fee == 300);
            }
            Err(e) => panic!("Error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_get_uniswap_v2_pool_data_batch_request() {
        dotenv::dotenv().ok();
        let rpc_endpoint = std::env::var("NETWORK_RPC").expect("Missing NETWORK_RPC env variable");
        let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint).unwrap());
        let addresses = vec![
            H160::from_str("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc").unwrap(), // WETH<>USDc
            H160::from_str("0x811beed0119b4afce20d2583eb608c6f7af1954f").unwrap(), // SHIB<>WETH
        ];

        let r = get_uniswap_v2_pool_data_batch_request(&addresses, 300, middleware.clone())
            .await
            .unwrap();

        let pool1 = &r[0];
        let pool2 = &r[1];

        {
            assert_eq!(pool1.address, addresses[0]);
            assert_eq!(
                pool1.token_a,
                H160::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap()
            );
            assert_eq!(pool1.token_a_decimals, 6);
            assert_eq!(
                pool1.token_b,
                H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap()
            );
            assert_eq!(pool1.token_b_decimals, 18);
            assert!(pool1.reserve_0 > 0);
            assert!(pool1.reserve_1 > 0);
            assert!(pool1.fee == 300);
        }

        {
            assert_eq!(pool2.address, addresses[1]);
            assert_eq!(
                pool2.token_a,
                H160::from_str("0x95ad61b0a150d79219dcf64e1e6cc01f0b64c4ce").unwrap()
            );
            assert_eq!(pool2.token_a_decimals, 18);
            assert_eq!(
                pool2.token_b,
                H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap()
            );
            assert_eq!(pool2.token_b_decimals, 18);
            assert!(pool2.reserve_0 > 0);
            assert!(pool2.reserve_1 > 0);
            assert!(pool2.fee == 300);
        }
    }

    #[tokio::test]
    async fn test_get_weth_value_in_pool_batch_request() {
        dotenv::dotenv().ok();
        let rpc_endpoint = std::env::var("NETWORK_RPC").expect("Missing NETWORK_RPC env variable");
        let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint).unwrap());
        let pool_address = H160::from_str("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc").unwrap();
        let weth_address = H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap();
        let factory_address = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap();
        let total = get_weth_value_in_pool_batch_request(
            vec![pool_address],
            weth_address,
            factory_address,
            middleware.clone(),
            None,
        )
        .await
        .unwrap();
        let pool = UniswapV2Pool::new_from_address(pool_address, 300, middleware.clone())
            .await
            .unwrap();
        let reserves = pool.get_reserves(middleware.clone()).await.unwrap();
        let v0 = reserves.0 as f64 / 1_000_000 as f64; // USDc reserves, we need to account for 6 decimals
        let v1 = reserves.1 as f64 / 1_000_000 as f64; // Weth reserves, we need to account for 18 decimals
        let v1: f64 = v1 / 1_000_000 as f64; // Done by dividing 3 times 10^6 to avoid overflows on f64
        let v1: f64 = v1 / 1_000_000 as f64;
        let price = pool.calculate_price(weth_address).unwrap(); // Price of 1 Weth in USDc
        let amount_in_wei = total[&pool_address].as_u128();
        let amount_in_weth = amount_in_wei as f64 / 1_000_000 as f64;
        let amount_in_weth = amount_in_weth / 1_000_000 as f64;
        let amount_in_weth = amount_in_weth / 1_000_000 as f64;
        assert!(v0 / price + v1 - amount_in_weth < 1e-10);
    }
}
