// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

interface IUniswapV2Pair {
    function decimals() external pure returns (uint8);

    function token0() external view returns (address);

    function token1() external view returns (address);

    function getReserves()
        external
        view
        returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
}

interface IUniswapV2Factory {
    function getPair(
        address tokenA,
        address tokenB
    ) external view returns (address pair);
}

contract GetWethValueInPoolBatchRequest {
    mapping(address => uint128) public tokenToWethPrices;

    constructor(address[] memory pools, address weth, address factory) {
        uint256[] memory wethValueInPools = new uint256[](pools.length);

        for (uint256 i = 0; i < pools.length; ++i) {
            address pool = pools[i];
            if (pool.code.length == 0) {
                wethValueInPools[i] = 0;
                continue;
            }

            address token0 = IUniswapV2Pair(pool).token0();
            address token1 = IUniswapV2Pair(pool).token1();

            if (token0.code.length == 0 || token1.code.length == 0) {
                wethValueInPools[i] = 0;
                continue;
            }

            (uint256 x, uint256 y) = getNormalisedReserves(pool);
            uint256 token0WethValueInPool = getWethEquivalentValueOfToken(
                token0,
                weth,
                x,
                factory
            );
            uint256 token1WethValueInPool = getWethEquivalentValueOfToken(
                token1,
                weth,
                y,
                factory
            );
            if (token0WethValueInPool != 0 && token1WethValueInPool != 0) {
                wethValueInPools[i] =
                    token0WethValueInPool +
                    token1WethValueInPool;
            } else {
                wethValueInPools[i] = 0;
            }
        }

        // insure abi encoding, not needed here but increase reusability for different return types
        // note: abi.encode add a first 32 bytes word with the address of the original data
        bytes memory abiEncodedData = abi.encode(wethValueInPools);

        assembly {
            // Return from the start of the data (discarding the original data address)
            // up to the end of the memory used
            let dataStart := add(abiEncodedData, 0x20)
            return(dataStart, sub(msize(), dataStart))
        }
    }

    function getNormalisedReserves(
        address pool
    ) internal returns (uint256 r_x, uint256 r_y) {
        (uint256 x, uint256 y, ) = IUniswapV2Pair(pool).getReserves();
        address token0 = IUniswapV2Pair(pool).token0();
        address token1 = IUniswapV2Pair(pool).token1();
        (uint8 token0Decimals, bool t0s) = getTokenDecimalsUnsafe(token0);
        (uint8 token1Decimals, bool t1s) = getTokenDecimalsUnsafe(token1);

        if (t0s && t1s) {
            r_x = token0Decimals <= 18
                ? x * (10 ** (18 - token0Decimals))
                : x / (10 ** (token0Decimals - 18));
            r_y = token1Decimals <= 18
                ? y * (10 ** (18 - token1Decimals))
                : y / (10 ** (token1Decimals - 18));
        }
    }

    function getBalanceOfUnsafe(
        address token,
        address targetAddress
    ) internal returns (uint256, bool) {
        (bool balanceOfSuccess, bytes memory balanceOfData) = token.call(
            abi.encodeWithSignature("balanceOf(address)", targetAddress)
        );
        if (!balanceOfSuccess) {
            return (0, false);
        }
        if (balanceOfData.length != 32) {
            return (0, false);
        }
        return (abi.decode(balanceOfData, (uint256)), true);
    }

    function getTokenDecimalsUnsafe(
        address token
    ) internal returns (uint8, bool) {
        (bool tokenDecimalsSuccess, bytes memory tokenDecimalsData) = token
            .call(abi.encodeWithSignature("decimals()"));
        if (!tokenDecimalsSuccess) {
            return (0, false);
        }
        if (tokenDecimalsData.length != 32) {
            return (0, false);
        }
        uint256 tokenDecimals = abi.decode(tokenDecimalsData, (uint256));
        if (tokenDecimals == 0 || tokenDecimals > 255) {
            return (0, false);
        }
        return (uint8(tokenDecimals), true);
    }

    function getWethEquivalentValueOfToken(
        address token,
        address weth,
        uint256 amount,
        address factory
    ) internal returns (uint256) {
        if (token == weth) {
            return amount;
        }
        uint128 tokenToWethPrice = tokenToWethPrices[token];
        if (tokenToWethPrice == 1) {
            return 0;
        }
        if (tokenToWethPrice != 0) {
            return mul64u(tokenToWethPrice, amount);
        }
        bool tokenIsToken0 = token < weth;
        address pairAddress = IUniswapV2Factory(factory).getPair(
            tokenIsToken0 ? token : weth,
            tokenIsToken0 ? weth : token
        );
        if (pairAddress == address(0)) {
            tokenToWethPrices[token] = 1;
            return 0;
        }
        (uint256 r_0, uint256 r_1) = getNormalisedReserves(pairAddress);
        uint128 price = divuu(
            tokenIsToken0 ? r_1 : r_0,
            tokenIsToken0 ? r_0 : r_1
        );
        tokenToWethPrices[token] = price;
        return mul64u(price, amount);
    }

    /// @notice helper function to multiply unsigned 64.64 fixed point number by a unsigned integer
    /// @param x 64.64 unsigned fixed point number
    /// @param y uint256 unsigned integer
    /// @return unsigned
    function mul64u(uint128 x, uint256 y) internal pure returns (uint256) {
        unchecked {
            if (y == 0 || x == 0) {
                return 0;
            }

            uint256 lo = (uint256(x) *
                (y & 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF)) >> 64;
            uint256 hi = uint256(x) * (y >> 128);

            require(
                hi <= 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF,
                "overflow-0 in mul64u"
            );
            hi <<= 64;

            require(
                hi <=
                    0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff -
                        lo,
                "overflow-1 in mul64u"
            );
            return hi + lo;
        }
    }

    /// @notice helper to divide two unsigned integers
    /// @param x uint256 unsigned integer
    /// @param y uint256 unsigned integer
    /// @return unsigned 64.64 fixed point number
    function divuu(uint256 x, uint256 y) internal pure returns (uint128) {
        unchecked {
            if (y == 0) return 0;

            uint256 answer;

            if (x <= 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF)
                answer = (x << 64) / y;
            else {
                uint256 msb = 192;
                uint256 xc = x >> 192;
                if (xc >= 0x100000000) {
                    xc >>= 32;
                    msb += 32;
                }
                if (xc >= 0x10000) {
                    xc >>= 16;
                    msb += 16;
                }
                if (xc >= 0x100) {
                    xc >>= 8;
                    msb += 8;
                }
                if (xc >= 0x10) {
                    xc >>= 4;
                    msb += 4;
                }
                if (xc >= 0x4) {
                    xc >>= 2;
                    msb += 2;
                }
                if (xc >= 0x2) msb += 1; // No need to shift xc anymore

                answer = (x << (255 - msb)) / (((y - 1) >> (msb - 191)) + 1);

                // require(
                //     answer <= 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF,
                //     "overflow in divuu"
                // );

                // We ignore pools that have a price that is too high because it is likely that the reserves are too low to be accurate
                // There is almost certainly not a pool that has a price of token/weth > 2^128
                if (answer > 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF) {
                    return 0;
                }

                uint256 hi = answer * (y >> 128);
                uint256 lo = answer * (y & 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF);

                uint256 xh = x >> 192;
                uint256 xl = x << 64;

                if (xl < lo) xh -= 1;
                xl -= lo; // We rely on overflow behavior here
                lo = hi << 128;
                if (xl < lo) xh -= 1;
                xl -= lo; // We rely on overflow behavior here

                assert(xh == hi >> 128);

                answer += xl / y;
            }

            // We ignore pools that have a price that is too high because it is likely that the reserves are too low to be accurate
            // There is almost certainly not a pool that has a price of token/weth > 2^128
            if (answer > 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF) {
                return 0;
            }

            return uint128(answer);
        }
    }
}
