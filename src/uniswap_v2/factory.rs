use std::sync::{Arc, Mutex};

use super::{batch_request, UniswapV2Pool};
use crate::errors::AMMError;
use ethers::prelude::abigen;
use ethers::{
    abi::RawLog,
    prelude::EthEvent,
    providers::Middleware,
    types::{BlockNumber, Filter, ValueOrArray, H160, H256, U256, U64},
};
use futures::future;
use indicatif::ProgressBar;
use serde::{Deserialize, Serialize};

abigen!(
    IUniswapV2Factory,
    r#"[
        function getPair(address tokenA, address tokenB) external view returns (address pair)
        function allPairs(uint256 index) external view returns (address)
        event PairCreated(address indexed token0, address indexed token1, address pair, uint256)
        function allPairsLength() external view returns (uint256)

    ]"#;
);

pub const PAIR_CREATED_EVENT_SIGNATURE: H256 = H256([
    13, 54, 72, 189, 15, 107, 168, 1, 52, 163, 59, 169, 39, 90, 197, 133, 217, 211, 21, 240, 173,
    131, 85, 205, 222, 253, 227, 26, 250, 40, 208, 233,
]);

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct UniswapV2Factory {
    pub address: H160,
    pub creation_block: u64,
    pub fee: u32,
}

impl UniswapV2Factory {
    pub fn new(address: H160, creation_block: u64, fee: u32) -> UniswapV2Factory {
        UniswapV2Factory {
            address,
            creation_block,
            fee,
        }
    }

    fn amm_created_event_signature(&self) -> H256 {
        PAIR_CREATED_EVENT_SIGNATURE
    }

    pub fn contract<M: Middleware>(&self, middleware: Arc<M>) -> IUniswapV2Factory<M> {
        IUniswapV2Factory::new(self.address, middleware)
    }

    pub async fn get_pair_address<M: Middleware>(
        &self,
        middleware: Arc<M>,
        token0: H160,
        token1: H160,
    ) -> H160 {
        self.contract(middleware)
            .get_pair(token0, token1)
            .call()
            .await
            .unwrap()
    }

    async fn get_pool_addresses_from_logs_range<M: Middleware>(
        &self,
        start_block: u64,
        end_block: u64,
        middleware: Arc<M>,
    ) -> Result<Vec<H160>, AMMError<M>> {
        let logs = middleware
            .get_logs(
                &Filter::new()
                    .topic0(ValueOrArray::Value(self.amm_created_event_signature()))
                    .address(self.address)
                    .from_block(BlockNumber::Number(U64([start_block])))
                    .to_block(BlockNumber::Number(U64([end_block]))),
            )
            .await
            .map_err(AMMError::MiddlewareError)?;

        let mut addresses = vec![];
        for log in logs {
            let pair_created_event: PairCreatedFilter =
                PairCreatedFilter::decode_log(&RawLog::from(log))?;
            addresses.push(pair_created_event.pair);
        }
        Ok(addresses)
    }

    async fn get_pools_from_addresses<M: Middleware>(
        &self,
        middleware: Arc<M>,
        addresses: Vec<H160>,
    ) -> Result<Vec<UniswapV2Pool>, AMMError<M>> {
        batch_request::get_uniswap_v2_pool_data_batch_request(&addresses, self.fee, middleware)
            .await
    }

    async fn get_pools_from_logs_range<M: Middleware>(
        &self,
        start_block: u64,
        end_block: u64,
        middleware: Arc<M>,
        progress_bar: Option<Arc<Mutex<ProgressBar>>>,
    ) -> Result<Vec<UniswapV2Pool>, AMMError<M>> {
        let addresses = self
            .get_pool_addresses_from_logs_range(start_block, end_block, middleware.clone())
            .await?;
        let pairs: Vec<UniswapV2Pool> =
            self.get_pools_from_addresses(middleware, addresses).await?;

        if let Some(progress_bar) = progress_bar {
            progress_bar.lock().unwrap().inc(end_block - start_block);
        }

        Ok(pairs)
    }

    async fn get_pool_addresses_range<M: Middleware>(
        &self,
        middleware: Arc<M>,
        from: u128,
        to: u128,
    ) -> Result<Vec<H160>, AMMError<M>> {
        batch_request::get_uniswap_v2_pairs_batch_request(
            self.address,
            U256::from(from),
            U256::from(to),
            middleware,
        )
        .await
    }

    async fn get_pools_range<M: Middleware>(
        &self,
        middleware: Arc<M>,
        from: u128,
        to: u128,
        progress_bar: Option<Arc<Mutex<ProgressBar>>>,
    ) -> Result<Vec<UniswapV2Pool>, AMMError<M>> {
        let addresses = self
            .get_pool_addresses_range(middleware.clone(), from, to)
            .await?;
        let pairs = batch_request::get_uniswap_v2_pool_data_batch_request(
            &addresses,
            self.fee,
            middleware.clone(),
        )
        .await?;

        if let Some(progress_bar) = progress_bar {
            let locked_pb = progress_bar.lock().unwrap();
            locked_pb.inc((to - from) as u64);
        }
        Ok(pairs)
    }

    pub async fn get_all_pools<M: Middleware>(
        &self,
        middleware: Arc<M>,
        step: Option<usize>,
    ) -> Result<(Vec<UniswapV2Pool>, u64), AMMError<M>> {
        let step = match step {
            Some(step) => step,
            None => 100,
        };
        let current_block = middleware
            .get_block_number()
            .await
            .map_err(AMMError::MiddlewareError)?
            .as_u64();
        let pairs_length: U256 = self
            .contract(middleware.clone())
            .all_pairs_length()
            .call()
            .await?;

        println!("Syncing {} uniswap pools", pairs_length);
        let pb = ProgressBar::new(pairs_length.as_u64());
        let shared_pb = Arc::new(Mutex::new(pb));

        let mut futures: Vec<_> = vec![];
        for i in (0..pairs_length.as_u128()).step_by(step) {
            futures.push(self.get_pools_range(
                middleware.clone(),
                i,
                (i + step as u128).min(pairs_length.as_u128()),
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

    pub async fn get_pools_from_logs<M: Middleware>(
        &self,
        middleware: Arc<M>,
        start_block: Option<u64>,
        end_block: Option<u64>,
        step: Option<usize>,
    ) -> Result<Vec<UniswapV2Pool>, AMMError<M>> {
        let start_block = match start_block {
            Some(start_block) => start_block,
            None => 0,
        };
        let end_block = match end_block {
            Some(end_block) => end_block,
            None => middleware
                .get_block_number()
                .await
                .map_err(AMMError::MiddlewareError)?
                .as_u64(),
        };
        let step = match step {
            Some(step) => step,
            None => 100,
        };
        let total_blocks = end_block - start_block;

        println!("Syncing uniswap pools for {} blocks", total_blocks);
        let pb = ProgressBar::new(total_blocks);
        let shared_pb = Arc::new(Mutex::new(pb));

        let mut futures = vec![];
        for i in (start_block..end_block).step_by(step) {
            futures.push(self.get_pools_from_logs_range(
                i,
                (i + step as u64).min(end_block),
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

        Ok(pools)
    }
}
