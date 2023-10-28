use std::{fs::read_to_string, sync::Arc};

use ethers::providers::Middleware;
use spinoff::{spinners, Color, Spinner};

use crate::errors::{AMMError, CheckpointError};

use super::{batch_request, factory::UniswapV2Factory, UniswapV2Pool};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub timestamp: usize,
    pub block_number: u64,
    pub factory: UniswapV2Factory,
    pub amms: Vec<UniswapV2Pool>,
}

impl Checkpoint {
    pub fn new(
        timestamp: usize,
        block_number: u64,
        factory: UniswapV2Factory,
        amms: Vec<UniswapV2Pool>,
    ) -> Checkpoint {
        Checkpoint {
            timestamp,
            block_number,
            factory,
            amms,
        }
    }

    pub fn read_from_path(path: &str) -> Result<Checkpoint, CheckpointError> {
        let checkpoint: Checkpoint = serde_json::from_str(read_to_string(path)?.as_str())?;
        Ok(checkpoint)
    }

    pub fn save_to_path(&self, path: &str) -> Result<(), CheckpointError> {
        std::fs::write(path, serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }
}

pub async fn sync_uniswap_v2_pools<M: Middleware>(
    factory: UniswapV2Factory,
    middleware: Arc<M>,
) -> Result<(Vec<UniswapV2Pool>, u64), AMMError<M>> {
    let spinner = Spinner::new(spinners::Dots, "Syncing Pools...", Color::Blue);
    let current_block = middleware
        .get_block_number()
        .await
        .map_err(AMMError::MiddlewareError)?
        .as_u64();

    let pairs = factory
        .get_all_pairs_addresses_via_batched_calls(middleware.clone(), Some(100))
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
