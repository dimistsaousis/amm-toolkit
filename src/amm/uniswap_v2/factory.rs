use std::sync::{Arc, Mutex};

use super::{batch_request, UniswapV2Pool};
use crate::errors::AMMError;
use ethers::prelude::abigen;
use ethers::{
    abi::RawLog,
    prelude::EthEvent,
    providers::Middleware,
    types::{BlockNumber, Filter, Log, ValueOrArray, H160, H256, U256, U64},
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

    pub async fn new_pool_from_log<M: Middleware>(
        &self,
        log: Log,
        middleware: Arc<M>,
    ) -> Result<UniswapV2Pool, AMMError<M>> {
        let pair_created_event: PairCreatedFilter =
            PairCreatedFilter::decode_log(&RawLog::from(log))?;
        Ok(UniswapV2Pool::new_from_address(pair_created_event.pair, self.fee, middleware).await?)
    }

    pub async fn get_all_pools_for_block_from_logs<M: Middleware>(
        &self,
        block: u64,
        middleware: Arc<M>,
    ) -> Result<Vec<UniswapV2Pool>, AMMError<M>> {
        let logs = middleware
            .get_logs(
                &Filter::new()
                    .topic0(ValueOrArray::Value(self.amm_created_event_signature()))
                    .address(self.address)
                    .from_block(BlockNumber::Number(U64([block])))
                    .to_block(BlockNumber::Number(U64([block]))),
            )
            .await
            .map_err(AMMError::MiddlewareError)?;

        let futures: Vec<_> = logs
            .into_iter()
            .map(|log| self.new_pool_from_log(log, middleware.clone()))
            .collect();

        let results: Vec<Result<UniswapV2Pool, AMMError<M>>> = future::join_all(futures).await;

        let mut pools = Vec::new();
        for result in results {
            match result {
                Ok(pool) => pools.push(pool),
                Err(err) => return Err(err),
            }
        }

        Ok(pools)
    }

    pub async fn get_pair_addresses_range<M: Middleware>(
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

    pub async fn get_pairs_range_from_addresses<M: Middleware>(
        &self,
        middleware: Arc<M>,
        addresses: Vec<H160>,
    ) -> Result<Vec<UniswapV2Pool>, AMMError<M>> {
        batch_request::get_uniswap_v2_pool_data_batch_request(&addresses, self.fee, middleware)
            .await
    }

    pub async fn get_pairs_range<M: Middleware>(
        &self,
        middleware: Arc<M>,
        from: u128,
        to: u128,
        progress_bar: Option<Arc<Mutex<ProgressBar>>>,
    ) -> Result<Vec<UniswapV2Pool>, AMMError<M>> {
        let addresses = self
            .get_pair_addresses_range(middleware.clone(), from, to)
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

    pub async fn get_all_pairs_addresses_via_batched_calls<M: Middleware>(
        &self,
        middleware: Arc<M>,
        pairs_length: Option<u32>,
    ) -> Result<Vec<H160>, AMMError<M>> {
        let mut pairs = vec![];
        let factory = IUniswapV2Factory::new(self.address, middleware.clone());
        let pairs_length: U256 = match pairs_length {
            Some(length) => U256::from(length),
            None => factory.all_pairs_length().call().await?,
        };
        let step = 766; //max batch size for this call until codesize is too large
        let mut idx_from = U256::zero();
        let mut idx_to = if step > pairs_length.as_usize() {
            pairs_length
        } else {
            U256::from(step)
        };

        for _ in (0..pairs_length.as_u128()).step_by(step) {
            pairs.append(
                &mut batch_request::get_uniswap_v2_pairs_batch_request(
                    self.address,
                    idx_from,
                    idx_to,
                    middleware.clone(),
                )
                .await?,
            );
            idx_from = idx_to;
            idx_to = (idx_to + step).min(pairs_length - 1);
        }
        Ok(pairs)
    }

    pub async fn get_all_pair_addresses<M: Middleware>(
        &self,
        middleware: Arc<M>,
    ) -> Result<Vec<H160>, AMMError<M>> {
        self.get_all_pairs_addresses_via_batched_calls(middleware, None)
            .await
    }
}
