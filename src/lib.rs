#![deny(missing_docs)]
//! a library for creating pim simulator

pub(crate) mod pim;
pub(crate) mod spm;
pub use pim::Simulator;
#[cfg(test)]
mod tests {
    use crate::{
        pim::{
            config::Config,
            level::{ddr4, LevelTrait},
        },
        Simulator,
    };

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    fn pim_test_impl(mut simulator: Simulator, config: &Config) {
        simulator.run(&config);
    }

    #[test]
    fn pim_test() {
        let config = Config::new("config.toml");
        match config.dram_type {
            crate::pim::config::DramType::DDR3 => todo!(),
            crate::pim::config::DramType::DDR4 => pim_test_impl(Simulator::new(&config), &config),
            crate::pim::config::DramType::LPDDR3 => todo!(),
            crate::pim::config::DramType::LPDDR4 => todo!(),
            crate::pim::config::DramType::HBM => todo!(),
            crate::pim::config::DramType::HBM2 => todo!(),
        }
    }
}
