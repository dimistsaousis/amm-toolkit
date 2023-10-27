use ethers::{
    providers::{Http, Provider},
    types::{H160, U256},
};
use std::{str::FromStr, sync::Arc};

use crate::amm::uniswap_v2::UniswapV2Pool;

pub async fn simulate_swaps() -> eyre::Result<()> {
    let rpc_endpoint = std::env::var("NETWORK_RPC").expect("Missing NETWORK_RPC env variable");
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
