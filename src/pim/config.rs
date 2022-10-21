use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub enum DramType {
    DDR3,
    DDR4,
    LPDDR3,
    LPDDR4,
    HBM,
    HBM2,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct LevelConfig {
    pub num: usize,
    pub merger_num: usize,
    pub max_msg_in: usize,
    pub max_msg_out: usize,
    pub max_msg_generated: usize,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    // memory config
    pub dram_type: DramType,
    pub subarray_provider_size: usize,
    pub subarray_task_queue_size: usize,
    pub subarrays: usize,
    pub precharge_cycle: u64,
    pub activate_cycle: u64,
    pub rows: usize,
    pub row_size: usize,
    pub columns: usize,
    pub graph_path: String,
    pub output_path: PathBuf,
    pub channels: LevelConfig,
    pub ranks: LevelConfig,
    pub chips: LevelConfig,
    pub bank_groups: LevelConfig,
    pub banks: LevelConfig,
}
impl Config {
    pub fn new(path: impl AsRef<Path>) -> Self {
        toml::from_str(std::fs::read_to_string(path).unwrap().as_str()).unwrap()
    }
}

impl Config {
    pub fn from_ddr4(channels: LevelConfig, ranks: LevelConfig) -> Self {
        Self {
            dram_type: DramType::DDR4,
            channels,
            ranks,
            rows: 32768,
            columns: 256,
            precharge_cycle: 2,
            activate_cycle: 2,
            subarrays: 16,
            row_size: 2,
            graph_path: "mtx/test.mtx".to_string(),
            output_path: PathBuf::from("output/ddr4.json"),
            chips: LevelConfig {
                num: 8,
                merger_num: 8,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
            bank_groups: LevelConfig {
                num: 8,
                merger_num: 8,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
            banks: LevelConfig {
                num: 8,
                merger_num: 8,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
            subarray_provider_size: 2,
            subarray_task_queue_size: 2,
        }
    }
    #[allow(dead_code, unreachable_code)]

    pub fn from_hbm() -> Self {
        Self { ..todo!() }
    }
    #[allow(dead_code, unreachable_code)]

    pub fn from_hbm2() -> Self {
        Self {
            dram_type: DramType::HBM2,

            ..todo!()
        }
    }
    pub fn save_to_file(&self, path: impl AsRef<Path>) {
        std::fs::write(path, toml::to_string_pretty(self).unwrap()).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn save_configs() {
        Config::from_ddr4(
            LevelConfig {
                num: 1,
                merger_num: 16,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
            LevelConfig {
                num: 2,
                merger_num: 12,
                max_msg_in: 2,
                max_msg_out: 2,
                max_msg_generated: 2,
            },
        )
        .save_to_file("ddr4.toml");
        // Config::from_hbm().save_to_file("hbm.toml");
        // Config::from_hbm2().save_to_file("hbm2.toml");
    }

    #[test]
    #[ignore]
    fn read_config() {
        let config = Config::new("ddr4.toml");
        println!("{:?}", config);
    }
}