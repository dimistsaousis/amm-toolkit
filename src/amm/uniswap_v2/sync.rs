use std::{fs::read_to_string, sync::Arc};

use super::{factory::UniswapV2Factory, UniswapV2Pool};
use crate::errors::{AMMError, CheckpointError};
use ethers::providers::Middleware;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub timestamp: usize,
    pub block_number: u64,
    pub factory: UniswapV2Factory,
    pub pools: Vec<UniswapV2Pool>,
}

impl Checkpoint {
    pub fn new(
        timestamp: usize,
        block_number: u64,
        factory: UniswapV2Factory,
        pools: Vec<UniswapV2Pool>,
    ) -> Checkpoint {
        Checkpoint {
            timestamp,
            block_number,
            factory,
            pools,
        }
    }

    pub fn read_from_path(path: &str) -> Result<Checkpoint, CheckpointError> {
        let path = format!("checkpoint_data/{}", path);
        let checkpoint: Checkpoint = serde_json::from_str(read_to_string(path)?.as_str())?;
        Ok(checkpoint)
    }

    pub fn save_to_path(&self, path: &str) -> Result<(), CheckpointError> {
        let path = format!("checkpoint_data/{}", path);
        std::fs::write(path, serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }
}

pub async fn sync_uniswap_v2_pools<M: Middleware>(
    factory: UniswapV2Factory,
    middleware: Arc<M>,
) -> Result<Vec<UniswapV2Pool>, AMMError<M>> {
    let checkpoint = Checkpoint::read_from_path("uniswap_v2_pairs");
    let pools = match checkpoint {
        Ok(_) => sync_uniswap_v2_pools_from_checkpoint(factory, middleware).await?,
        _ => sync_uniswap_v2_pools_no_checkpoint(factory, middleware).await?,
    };
    Ok(pools)
}

async fn sync_uniswap_v2_pools_no_checkpoint<M: Middleware>(
    factory: UniswapV2Factory,
    middleware: Arc<M>,
) -> Result<Vec<UniswapV2Pool>, AMMError<M>> {
    let (pools, block_number) = factory.get_all_pools(middleware, None).await?;
    Checkpoint::new(
        chrono::Utc::now().timestamp() as usize,
        block_number,
        factory,
        pools.clone(),
    )
    .save_to_path("uniswap_v2_pairs")?;
    Ok(pools)
}

async fn sync_uniswap_v2_pools_from_checkpoint<M: Middleware>(
    factory: UniswapV2Factory,
    middleware: Arc<M>,
) -> Result<Vec<UniswapV2Pool>, AMMError<M>> {
    let mut checkpoint = Checkpoint::read_from_path("uniswap_v2_pairs")?;
    let end_block = middleware
        .get_block_number()
        .await
        .map_err(AMMError::MiddlewareError)?
        .as_u64();
    let mut new_pools = factory
        .get_pools_from_logs(
            middleware,
            Some(checkpoint.block_number + 1),
            Some(end_block),
            None,
        )
        .await?;
    checkpoint.pools.append(&mut new_pools);
    checkpoint.block_number = end_block;
    checkpoint.timestamp = chrono::Utc::now().timestamp() as usize;
    checkpoint.save_to_path("uniswap_v2_pairs")?;
    Ok(new_pools)
}
