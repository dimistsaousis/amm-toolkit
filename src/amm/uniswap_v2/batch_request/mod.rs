use std::sync::Arc;

use ethers::{
    abi::{ParamType, Token},
    providers::Middleware,
    types::{Bytes, H160, U256},
};

use crate::errors::AMMError;

use super::UniswapV2Pool;

use ethers::prelude::abigen;

abigen!(
    IGetUniswapV2PoolDataBatchRequest,
    "src/amm/uniswap_v2/batch_request/GetUniswapV2PoolDataBatchRequest.json";
    IGetUniswapV2PairsBatchRequest,
    "src/amm/uniswap_v2/batch_request/GetUniswapV2PairsBatchRequest.json",
);

pub async fn get_uniswap_v2_pool_data_batch_request_single<M: Middleware>(
    pair_address: H160,
    fee: u32,
    middleware: Arc<M>,
) -> Result<UniswapV2Pool, AMMError<M>> {
    let constructor_args = Token::Tuple(vec![Token::Array(vec![Token::Address(pair_address)])]);
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

    let tokens = return_data_tokens
        .into_iter()
        .next()
        .ok_or(AMMError::<M>::BatchRequestError(pair_address))?
        .into_array()
        .ok_or(AMMError::<M>::BatchRequestError(pair_address))?
        .into_iter()
        .next()
        .ok_or(AMMError::<M>::BatchRequestError(pair_address))?
        .into_tuple()
        .ok_or(AMMError::<M>::BatchRequestError(pair_address))?;

    let pool = UniswapV2Pool {
        token_a: tokens[0]
            .to_owned()
            .into_address()
            .ok_or(AMMError::<M>::BatchRequestError(pair_address))?,
        token_a_decimals: tokens[1]
            .to_owned()
            .into_uint()
            .ok_or(AMMError::<M>::BatchRequestError(pair_address))?
            .as_u32() as u8,
        token_b: tokens[2]
            .to_owned()
            .into_address()
            .ok_or(AMMError::<M>::BatchRequestError(pair_address))?,
        token_b_decimals: tokens[3]
            .to_owned()
            .into_uint()
            .ok_or(AMMError::<M>::BatchRequestError(pair_address))?
            .as_u32() as u8,
        reserve_0: tokens[4]
            .to_owned()
            .into_uint()
            .ok_or(AMMError::<M>::BatchRequestError(pair_address))?
            .as_u128(),
        reserve_1: tokens[5]
            .to_owned()
            .into_uint()
            .ok_or(AMMError::<M>::BatchRequestError(pair_address))?
            .as_u128(),
        address: pair_address,
        fee,
    };

    Ok(pool)
}

pub async fn get_uniswap_v2_pool_data_batch_request<M: Middleware>(
    pair_addresses: &Vec<H160>,
    fee: u32,
    middleware: Arc<M>,
) -> Result<Vec<UniswapV2Pool>, AMMError<M>> {
    let mut target_addresses = vec![];
    for address in pair_addresses.iter() {
        target_addresses.push(Token::Address(*address));
    }
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

    let mut pools = vec![];
    return_data_tokens
        .into_iter()
        .next()
        .unwrap()
        .into_array()
        .unwrap()
        .into_iter()
        .enumerate()
        .for_each(|(idx, token)| {
            let tup = token.into_tuple().unwrap();
            let pair_address = pair_addresses[idx];
            pools.push(UniswapV2Pool {
                token_a: tup[0]
                    .to_owned()
                    .into_address()
                    .ok_or(AMMError::<M>::BatchRequestError(pair_address))
                    .unwrap(),
                token_a_decimals: tup[1]
                    .to_owned()
                    .into_uint()
                    .ok_or(AMMError::<M>::BatchRequestError(pair_address))
                    .unwrap()
                    .as_u32() as u8,
                token_b: tup[2]
                    .to_owned()
                    .into_address()
                    .ok_or(AMMError::<M>::BatchRequestError(pair_address))
                    .unwrap(),
                token_b_decimals: tup[3]
                    .to_owned()
                    .into_uint()
                    .ok_or(AMMError::<M>::BatchRequestError(pair_address))
                    .unwrap()
                    .as_u32() as u8,
                reserve_0: tup[4]
                    .to_owned()
                    .into_uint()
                    .ok_or(AMMError::<M>::BatchRequestError(pair_address))
                    .unwrap()
                    .as_u128(),
                reserve_1: tup[5]
                    .to_owned()
                    .into_uint()
                    .ok_or(AMMError::<M>::BatchRequestError(pair_address))
                    .unwrap()
                    .as_u128(),
                address: pair_address,
                fee,
            });
        });

    Ok(pools)
}

pub async fn get_uniswap_v2_pairs_batch_request<M: Middleware>(
    factory_address: H160,
    from: U256,
    step: U256,
    middleware: Arc<M>,
) -> Result<Vec<H160>, AMMError<M>> {
    let mut pairs = vec![];
    let constructor_args = Token::Tuple(vec![
        Token::Uint(from),
        Token::Uint(step),
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
}
