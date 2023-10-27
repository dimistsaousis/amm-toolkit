pub mod batch_request;
use std::sync::Arc;

use ethers::{
    prelude::abigen,
    providers::Middleware,
    types::{H160, U256},
};
use serde::{Deserialize, Serialize};

use crate::{
    errors::{AMMError, ArithmeticError, SwapSimulationError},
    large_int_maths::{div_uu, q64_to_f64, U128_0X10000000000000000},
};

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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        address: H160,
        token_a: H160,
        token_a_decimals: u8,
        token_b: H160,
        token_b_decimals: u8,
        reserve_0: u128,
        reserve_1: u128,
        fee: u32,
    ) -> UniswapV2Pool {
        UniswapV2Pool {
            address,
            token_a,
            token_a_decimals,
            token_b,
            token_b_decimals,
            reserve_0,
            reserve_1,
            fee,
        }
    }

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

    //Creates a new instance of the pool from the pair address, and syncs the pool data
    pub async fn new_from_address<M: Middleware>(
        pair_address: H160,
        fee: u32,
        middleware: Arc<M>,
    ) -> Result<Self, AMMError<M>> {
        let pool =
            batch_request::get_uniswap_v2_pool_data_batch_request(pair_address, fee, middleware)
                .await?;

        if !pool.data_is_populated() {
            return Err(AMMError::PoolDataError);
        }
        Ok(pool)
    }
}
