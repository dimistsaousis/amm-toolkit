use ethers::types::{H160, U256};
use std::str::FromStr;

use crate::{
    configs::Config,
    uniswap_v2::{
        batch_request::get_weth_value_in_pools, sync::sync_uniswap_v2_pools, UniswapV2Pool,
    },
};

pub async fn simulate_swaps() -> eyre::Result<()> {
    let config = Config::new()?;
    let pool = UniswapV2Pool::new_from_address(
        config.uniswap_v2_pairs["WETH"]["USDc"],
        300,
        config.middleware,
    )
    .await?;
    let amount_out = pool.simulate_swap(
        config.tokens["WETH"],
        U256::from_dec_str("1000000000000000000")?,
    )?;
    println!("Amount out: {amount_out}");
    Ok(())
}

pub async fn get_usdc_weth_price() -> eyre::Result<()> {
    let config = Config::new()?;
    let price = config
        .pool("WETH", "USDc")
        .await?
        .calculate_price(config.tokens["WETH"])?;
    println!("ETH/USD: {price}");
    Ok(())
}

pub async fn get_swap_call_data() -> eyre::Result<()> {
    let config = Config::new()?;
    let to_address = H160::from_str("0xdfC9F6651b27Fe326D61052979e6d8b90774240d")?;
    let swap_calldata = config.pool("WETH", "USDc").await?.swap_calldata(
        U256::from(10000),
        U256::zero(),
        to_address,
        vec![],
    )?;
    println!("Swap calldata: {:?}", swap_calldata);
    Ok(())
}

pub async fn get_pools_from_log() -> eyre::Result<()> {
    let config = Config::new()?;
    let pools = config
        .uniswap_v2_factory
        .get_pools_from_logs(
            config.middleware,
            Some(10008355),
            Some(10008355 + 1000),
            None,
        )
        .await?;
    println!("Got following pools from blocks: {:?}", pools);
    Ok(())
}

pub async fn get_all_pools() -> eyre::Result<()> {
    let config = Config::new()?;
    let pools = config
        .uniswap_v2_factory
        .get_all_pools(config.middleware, None)
        .await?;
    println!("Got {:?}", pools.0.len());
    Ok(())
}

pub async fn run_sync_uniswap_v2_pools() -> eyre::Result<()> {
    let config = Config::new()?;
    let pools = sync_uniswap_v2_pools(config.uniswap_v2_factory, config.middleware).await?;
    println!("Got {:?}", pools.len());
    Ok(())
}

pub async fn get_top10_pools_in_terms_of_weth_equivalent_value() -> eyre::Result<()> {
    let config = Config::new()?;
    let pools =
        sync_uniswap_v2_pools(config.uniswap_v2_factory.clone(), config.middleware.clone()).await?;
    let pool_addresses = pools.into_iter().map(|pool| pool.address).collect();
    let weth_values_in_pools_batch = get_weth_value_in_pools(
        pool_addresses,
        config.tokens["WETH"],
        config.uniswap_v2_factory.address,
        config.middleware.clone(),
        Some(50),
    )
    .await?;
    println!("Got {} number of pools", weth_values_in_pools_batch.len());
    Ok(())
}
