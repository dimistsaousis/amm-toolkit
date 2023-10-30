use amm_toolkit::playground;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    playground::simulate_swaps().await.unwrap();
    playground::get_usdc_weth_price().await.unwrap();
    playground::get_swap_call_data().await.unwrap();
    playground::get_pools_from_log().await.unwrap();
    playground::get_all_pools().await.unwrap();
    playground::run_sync_uniswap_v2_pools().await.unwrap();
}
