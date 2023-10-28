use std::sync::Arc;

use ethers::providers::Middleware;
use spinoff::{spinners, Color, Spinner};

use crate::errors::AMMError;

use super::{batch_request, factory::UniswapV2Factory, UniswapV2Pool};

pub async fn sync_uniswap_v2_pools<M: Middleware>(
    factory: UniswapV2Factory,
    middleware: Arc<M>,
) -> Result<(Vec<UniswapV2Pool>, u64), AMMError<M>> {
    let spinner = Spinner::new(spinners::Dots, "Syncing Poolss...", Color::Blue);
    let current_block = middleware
        .get_block_number()
        .await
        .map_err(AMMError::MiddlewareError)?
        .as_u64();

    let pairs = factory
        .get_all_pairs_addresses_via_batched_calls(middleware.clone(), Some(200))
        .await
        .unwrap();
    let pools = batch_request::get_uniswap_v2_pool_data_batch_request(
        &pairs,
        factory.fee,
        middleware.clone(),
    )
    .await?;
    spinner.success("Pools synced");
    Ok((pools, current_block))
}
