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
