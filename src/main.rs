use amm_toolkit::playground;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    playground::simulate_swaps().await.unwrap();
    playground::get_usdc_weth_price().await.unwrap();
    playground::get_swap_call_data().await.unwrap();
    playground::get_pairs_of_uniswap_v2_factory().await.unwrap();
    playground::get_all_uniswap_v2_pools_for_block_from_logs()
        .await
        .unwrap();
    // playground::compare_concurrent_and_serial_uniswap2_pool_sync()
    //     .await
    //     .unwrap();
    playground::get_sync_all_uniswap_v2_pairs_from_logs()
        .await
        .unwrap();
}
