use std::sync::Arc;

use ethers::{
    prelude::abigen,
    providers::Middleware,
    types::{H160, U256},
};
use serde::{Deserialize, Serialize};

use crate::errors::AMMError;

use super::batch_request;

abigen!(
    IUniswapV2Factory,
    r#"[
        function getPair(address tokenA, address tokenB) external view returns (address pair)
        function allPairs(uint256 index) external view returns (address)
        event PairCreated(address indexed token0, address indexed token1, address pair, uint256)
        function allPairsLength() external view returns (uint256)

    ]"#;
);

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
}
