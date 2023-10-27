pub mod batch_request;
use std::sync::Arc;

use ethers::{
    abi::{Bytes, Token},
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

    pub fn simulate_swap(
        &self,
        token_in: H160,
        amount_in: U256,
    ) -> Result<U256, SwapSimulationError> {
        if self.token_a == token_in {
            Ok(self.get_amount_out(
                amount_in,
                U256::from(self.reserve_0),
                U256::from(self.reserve_1),
            ))
        } else {
            Ok(self.get_amount_out(
                amount_in,
                U256::from(self.reserve_1),
                U256::from(self.reserve_0),
            ))
        }
    }

    pub fn get_amount_out(&self, amount_in: U256, reserve_in: U256, reserve_out: U256) -> U256 {
        if amount_in.is_zero() || reserve_in.is_zero() || reserve_out.is_zero() {
            return U256::zero();
        }
        let fee = (10000 - (self.fee / 10)) / 10; //Fee of 300 => (10,000 - 30) / 10  = 997
        let amount_in_with_fee = amount_in * U256::from(fee);
        let numerator = amount_in_with_fee * reserve_out;
        let denominator = reserve_in * U256::from(1000) + amount_in_with_fee;

        numerator / denominator
    }

    pub fn calculate_price_64_x_64(&self, base_token: H160) -> Result<u128, ArithmeticError> {
        let decimal_shift = self.token_a_decimals as i8 - self.token_b_decimals as i8;

        let (r_0, r_1) = if decimal_shift < 0 {
            (
                U256::from(self.reserve_0)
                    * U256::from(10u128.pow(decimal_shift.unsigned_abs() as u32)),
                U256::from(self.reserve_1),
            )
        } else {
            (
                U256::from(self.reserve_0),
                U256::from(self.reserve_1) * U256::from(10u128.pow(decimal_shift as u32)),
            )
        };

        if base_token == self.token_a {
            if r_0.is_zero() {
                Ok(U128_0X10000000000000000)
            } else {
                div_uu(r_1, r_0)
            }
        } else if r_1.is_zero() {
            Ok(U128_0X10000000000000000)
        } else {
            div_uu(r_0, r_1)
        }
    }

    pub fn calculate_price(&self, base_token: H160) -> Result<f64, ArithmeticError> {
        Ok(q64_to_f64(self.calculate_price_64_x_64(base_token)?))
    }

    pub fn swap_calldata(
        &self,
        amount_0_out: U256,
        amount_1_out: U256,
        to: H160,
        calldata: Vec<u8>,
    ) -> Result<Bytes, ethers::abi::Error> {
        let input_tokens = vec![
            Token::Uint(amount_0_out),
            Token::Uint(amount_1_out),
            Token::Address(to),
            Token::Bytes(calldata),
        ];

        IUNISWAPV2PAIR_ABI
            .function("swap")?
            .encode_input(&input_tokens)
    }
}
