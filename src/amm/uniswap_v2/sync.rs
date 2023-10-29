use std::{
    fs::read_to_string,
    sync::{Arc, Mutex},
};

use ethers::{providers::Middleware, types::U256};
use futures::future;
use indicatif::ProgressBar;

use super::{factory::UniswapV2Factory, UniswapV2Pool};
use crate::errors::{AMMError, CheckpointError};
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

pub async fn sync_all_uniswap_v2_pools_concurrent<M: Middleware>(
    factory: UniswapV2Factory,
    middleware: Arc<M>,
) -> Result<(Vec<UniswapV2Pool>, u64), AMMError<M>> {
    let current_block = middleware
        .get_block_number()
        .await
        .map_err(AMMError::MiddlewareError)?
        .as_u64();

    let pairs_length: U256 = factory
        .contract(middleware.clone())
        .all_pairs_length()
        .call()
        .await?;
    println!("Syncing {} uniswap pools", pairs_length);
    let mut futures: Vec<_> = vec![];
    let pb = ProgressBar::new(pairs_length.as_u64());
    let shared_pb = Arc::new(Mutex::new(pb));

    for i in (0..pairs_length.as_u128()).step_by(100) {
        futures.push(factory.get_pairs_range(
            middleware.clone(),
            i,
            (i + 100).min(pairs_length.as_u128()),
            Some(shared_pb.clone()),
        ));
    }

    let results: Vec<Result<Vec<UniswapV2Pool>, AMMError<M>>> = future::join_all(futures).await;

    let mut pools = Vec::new();
    for result in results {
        match result {
            Ok(mut pool_batch) => pools.append(&mut pool_batch),
            Err(err) => return Err(err),
        }
    }

    shared_pb.lock().unwrap().finish();

    Ok((pools, current_block))
}

pub async fn sync_all_uniswap_v2_pools_serial<M: Middleware>(
    factory: UniswapV2Factory,
    middleware: Arc<M>,
) -> Result<(Vec<UniswapV2Pool>, u64), AMMError<M>> {
    let current_block = middleware
        .get_block_number()
        .await
        .map_err(AMMError::MiddlewareError)?
        .as_u64();

    let pairs_length: U256 = factory
        .contract(middleware.clone())
        .all_pairs_length()
        .call()
        .await?;
    println!("Syncing {} uniswap pools", pairs_length);
    let pb = ProgressBar::new(pairs_length.as_u64());

    let mut pools = vec![];

    for i in (0..pairs_length.as_u128()).step_by(100) {
        let mut pools_ = factory
            .get_pairs_range(
                middleware.clone(),
                i,
                (i + 100).min(pairs_length.as_u128()),
                None,
            )
            .await?;
        pools.append(&mut pools_);
        pb.inc(100);
    }

    pb.finish();

    Ok((pools, current_block))
}

pub async fn sync_all_uniswap_v2_pools_from_logs_use_range<M: Middleware>(
    factory: UniswapV2Factory,
    middleware: Arc<M>,
    sync_block: u64,
) -> Result<(Vec<UniswapV2Pool>, u64), AMMError<M>> {
    let current_block = middleware
        .get_block_number()
        .await
        .map_err(AMMError::MiddlewareError)?
        .as_u64();

    let total_blocks = current_block - sync_block;
    println!("Syncing uniswap pools for {} blocks", total_blocks);

    let mut futures: Vec<_> = vec![];
    let pb = ProgressBar::new(total_blocks);
    let shared_pb = Arc::new(Mutex::new(pb));

    for i in (sync_block..current_block).step_by(100) {
        futures.push(factory.get_all_pools_for_block_range_from_logs(
            i,
            (i + 100).min(current_block),
            middleware.clone(),
            Some(shared_pb.clone()),
        ));
    }

    let results: Vec<Result<Vec<UniswapV2Pool>, AMMError<M>>> = future::join_all(futures).await;
    let mut pools = Vec::new();
    for result in results {
        match result {
            Ok(mut pool_batch) => pools.append(&mut pool_batch),
            Err(AMMError::PoolDataError(addr)) => {
                println!("Data not populated for {:?}", addr);
            }
            Err(err) => return Err(err),
        }
    }

    shared_pb.lock().unwrap().finish();

    Ok((pools, current_block))
}
