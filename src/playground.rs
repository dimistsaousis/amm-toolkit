use ethers::{
    providers::{Http, Provider},
    types::{H160, U256},
};
use std::{str::FromStr, sync::Arc};

use crate::amm::uniswap_v2::{
    factory::UniswapV2Factory, sync::sync_all_uniswap_v2_pools, UniswapV2Pool,
};

pub async fn simulate_swaps() -> eyre::Result<()> {
    let rpc_endpoint: String =
        std::env::var("NETWORK_RPC").expect("Missing NETWORK_RPC env variable");
    let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint)?);
    let uniswap_v2_usdc_weth_pool_address =
        H160::from_str("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc")?;
    let pool =
        UniswapV2Pool::new_from_address(uniswap_v2_usdc_weth_pool_address, 300, middleware).await?;
    let token_in = H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?;
    let amount_out = pool.simulate_swap(token_in, U256::from_dec_str("1000000000000000000")?)?;
    println!("Amount out: {amount_out}");
    Ok(())
}

pub async fn get_usdc_weth_price() -> eyre::Result<()> {
    let rpc_endpoint = std::env::var("NETWORK_RPC").expect("Missing NETWORK_RPC env variable");
    let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint)?);
    let uniswap_v2_usdc_weth_pool_address =
        H160::from_str("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc")?;
    let pool =
        UniswapV2Pool::new_from_address(uniswap_v2_usdc_weth_pool_address, 300, middleware).await?;
    let token_in = H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?;
    let price = pool.calculate_price(token_in)?;
    println!("ETH/USD: {price}");
    Ok(())
}

pub async fn get_swap_call_data() -> eyre::Result<()> {
    let rpc_endpoint = std::env::var("NETWORK_RPC")?;
    let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint)?);
    let uniswap_v2_usdc_weth_pool_address =
        H160::from_str("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc")?;
    let pool =
        UniswapV2Pool::new_from_address(uniswap_v2_usdc_weth_pool_address, 300, middleware).await?;
    let to_address = H160::from_str("0xdfC9F6651b27Fe326D61052979e6d8b90774240d")?;
    let swap_calldata = pool.swap_calldata(U256::from(10000), U256::zero(), to_address, vec![]);
    println!("Swap calldata: {:?}", swap_calldata);
    Ok(())
}

pub async fn get_pairs_of_uniswap_v2_factory() -> eyre::Result<()> {
    let rpc_endpoint = std::env::var("NETWORK_RPC")?;
    let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint)?);
    let uniswap_v2_factory = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f")?;
    let factory = UniswapV2Factory::new(uniswap_v2_factory, 2638438, 300);
    let pair_addresses = factory
        .get_all_pairs_addresses_via_batched_calls(middleware, Some(200))
        .await?;
    println!("Got *{}* pair addresses", pair_addresses.len());
    Ok(())
}

pub async fn get_sync_uniswap_v2_pools() -> eyre::Result<()> {
    let rpc_endpoint = std::env::var("NETWORK_RPC")?;
    let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint)?);
    let uniswap_v2_factory = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f")?;
    let factory = UniswapV2Factory::new(uniswap_v2_factory, 2638438, 300);
    let (pools, _) = sync_all_uniswap_v2_pools(factory, middleware)
        .await
        .unwrap();
    println!("Got *{}* pools addresses", pools.len());
    Ok(())
}

pub async fn get_all_uniswap_v2_pools_for_block_from_logs() -> eyre::Result<()> {
    let rpc_endpoint = std::env::var("NETWORK_RPC")?;
    let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint)?);
    let uniswap_v2_factory = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f")?;
    let factory = UniswapV2Factory::new(uniswap_v2_factory, 2638438, 300);
    let pools = factory
        .get_all_pools_for_block_from_logs(10008355, middleware)
        .await
        .unwrap();
    println!("Got {:?}", pools);
    Ok(())
}
