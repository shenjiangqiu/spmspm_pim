pub mod row_cycle;
pub mod jump;
use serde::{Deserialize, Serialize};
use sprs::{num_kinds::Pattern, CsMatI};
use tracing::info;

use crate::pim::configv2::ConfigV3;

use super::translate_mapping::TranslateMapping;

pub mod real_jump;
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
        matrix_tri_translated: &CsMatI<Pattern, u32>,
    ) -> eyre::Result<Self::R>;
}
