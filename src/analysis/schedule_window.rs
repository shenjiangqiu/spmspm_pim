//! create a method to schedule multple tasks that in a window event they have confilicts
//! - inside the window, when they have conflicts, they will try to share the stream
//! - outside the window, they will try to use the duplicated stream
//! - otherwise, they will try to run partial merger
use crate::pim::{
    config::Config,
    level::{ddr4, LevelTrait},
};

pub fn compute_window(config: &Config) {
    match config.dram_type {
        crate::pim::config::DramType::DDR3 => todo!(),
        crate::pim::config::DramType::DDR4 => {
            let total_size = ddr4::Storage::new(
                config.channels.num,
                config.ranks.num,
                config.chips.num,
                config.bank_groups.num,
                config.banks.num,
                config.subarrays,
                config.rows,
                config.columns,
            );
            compute_window_inner::<ddr4::Level>(config, &total_size)
        }
        crate::pim::config::DramType::LPDDR3 => todo!(),
        crate::pim::config::DramType::LPDDR4 => todo!(),
        crate::pim::config::DramType::HBM => todo!(),
        crate::pim::config::DramType::HBM2 => todo!(),
    }
}

fn compute_window_inner<LevelType: LevelTrait>(config: &Config, total_size: &LevelType::Storage) {
    todo!()
}

enum EventType {}
