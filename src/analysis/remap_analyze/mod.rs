pub mod action;
pub mod jump;
pub mod real_jump;
pub mod remote_updator;
// pub mod real_jump_iterative;
pub mod row_cycle;
use serde::{Deserialize, Serialize};
use sprs::{num_kinds::Pattern, CsMatViewI};
use tracing::info;

use crate::{algorithms::SpmvAlgorithm, pim::configv2::ConfigV3};

use super::translate_mapping::TranslateMapping;

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub enum SimulationType {
    #[default]
    RealJump,
}
pub fn run_simulation(config: ConfigV3) -> eyre::Result<()> {
    let current_time = std::time::Instant::now();
    info!("analyze with config: {:?}", config);

    match config.analysis {
        SimulationType::RealJump => {
            real_jump::run_simulation(config)?;
        }
    }
    info!("analyze finished in {:?}", current_time.elapsed());
    Ok(())
}

pub trait Simulator {
    type R;
    fn run(
        &mut self,
        mapping: &impl TranslateMapping,
        matrix_tri_translated: CsMatViewI<Pattern, u32>,
        algorithm: impl SpmvAlgorithm,
        max_rounds: usize,
    ) -> eyre::Result<Self::R>;
}

pub trait IterativeSimulator {
    type R;
    fn run(
        &mut self,
        mapping: &impl TranslateMapping,
        matrix_tri_translated: CsMatViewI<Pattern, u32>,
        algorithm: &mut impl SpmvAlgorithm,
    ) -> eyre::Result<Self::R>;
}
