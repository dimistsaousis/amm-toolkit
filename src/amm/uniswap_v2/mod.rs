use std::sync::Arc;

use async_trait::async_trait;
use dotenv::Error;
use ethers::{
    prelude::abigen,
    providers::{Middleware, MiddlewareError},
    types::H160,
};
use serde::{Deserialize, Serialize};

use crate::errors::AMMError;

use super::AutomatedMarketMaker;

abigen!(
    IUniswapV2Pair,
    r#"[
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
        function token0() external view returns (address)
        function token1() external view returns (address)
        function swap(uint256 amount0Out, uint256 amount1Out, address to, bytes calldata data);
        event Sync(uint112 reserve0, uint112 reserve1)
    ]"#;

    IErc20,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function decimals() external view returns (uint8)
    ]"#;
);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UniswapV2Pool {
    pub address: H160,
    pub token_a: H160,
    pub token_a_decimals: u8,
    pub token_b: H160,
    pub token_b_decimals: u8,
    pub reserve_0: u128,
    pub reserve_1: u128,
    pub fee: u32,
}

impl UniswapV2Pool {
    pub async fn get_reserves<M: Middleware>(
        &self,
        middleware: Arc<M>,
    ) -> Result<(u128, u128), AMMError<M>> {
        let pair = IUniswapV2Pair::new(self.address, middleware);
        let (r0, r1, _) = match pair.get_reserves().call().await {
            Ok(result) => result,
            Err(contract_error) => return Err(AMMError::ContractError(contract_error)),
        };

        Ok((r0, r1))
    }

    pub fn data_is_populated(&self) -> bool {
        !(self.token_a.is_zero()
            || self.token_b.is_zero()
            || self.reserve_0 == 0
            || self.reserve_1 == 0)
    }
}
