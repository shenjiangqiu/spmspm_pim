use std::path::Path;

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

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    // memory config
    pub dram_type: DramType,
    pub channels: usize,
    pub ranks: usize,
    pub chips: usize,
    pub bank_groups: usize,
    pub banks: usize,
    pub bank_provider_size: usize,
    pub bank_task_queue_size: usize,
    pub precharge_cycle: u64,
    pub activate_cycle: u64,
    pub subarrays: usize,
    pub rows: usize,
    pub row_size: usize,
    pub columns: usize,

    // pe configuration
    pub pe_num: usize,

    pub graph_path: String,
}
impl Config {
    pub fn new(path: impl AsRef<Path>) -> Self {
        toml::from_str(std::fs::read_to_string(path).unwrap().as_str()).unwrap()
    }
}

impl Config {
    pub fn from_ddr4(channels: usize, ranks: usize, pe_num: usize) -> Self {
        Self {
            dram_type: DramType::DDR4,
            channels,
            ranks,
            chips: 8,
            bank_groups: 4,
            banks: 4,
            rows: 32768,
            columns: 256,
            pe_num,
            bank_provider_size: 2,
            bank_task_queue_size: 2,
            precharge_cycle: 2,
            activate_cycle: 2,
            subarrays: 16,
            row_size: 2,
            graph_path: "data/roadNet-CA.mtx".to_string(),
        }
    }
    pub fn from_hbm() -> Self {
        Self {
            dram_type: DramType::HBM,
            channels: 16,
            ranks: 1,
            chips: 1,
            bank_groups: 1,
            banks: 1,
            rows: 32768,
            columns: 256,
            pe_num: 1,
            bank_provider_size: todo!(),
            bank_task_queue_size: todo!(),
            precharge_cycle: todo!(),
            activate_cycle: todo!(),
            subarrays: 16,
            row_size: todo!(),
            ..todo!()
        }
    }
    pub fn from_hbm2() -> Self {
        Self {
            dram_type: DramType::HBM2,
            channels: 16,
            ranks: 1,
            chips: 1,
            bank_groups: 1,
            banks: 1,
            rows: 32768,
            columns: 256,
            pe_num: 1,
            bank_provider_size: todo!(),
            bank_task_queue_size: todo!(),
            precharge_cycle: todo!(),
            activate_cycle: todo!(),
            subarrays: 16,
            row_size: todo!(),
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
        Config::from_ddr4(2, 2, 10).save_to_file("ddr4.toml");
        Config::from_hbm().save_to_file("hbm.toml");
        Config::from_hbm2().save_to_file("hbm2.toml");
    }

    #[test]
    #[ignore]
    fn read_config() {
        let config = Config::new("ddr4.toml");
        println!("{:?}", config);
    }
}
