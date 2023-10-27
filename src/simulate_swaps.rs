use ethers::{
    providers::{Http, Provider},
    types::H160,
};
use std::{str::FromStr, sync::Arc};

async fn simulate_swaps() {
    let rpc_endpoint = std::env::var("NETWORK_RPC").expect("Missing NETWORK_RPC env variable");
    let middleware = Arc::new(Provider::<Http>::try_from(rpc_endpoint));

    let uniswap_v2_usdc_weth_pool_address =
        H160::from_str("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");
}
