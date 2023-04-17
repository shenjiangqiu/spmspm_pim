use serde::{Deserialize, Serialize};
use sprs::{num_kinds::Pattern, TriMatI};
use tracing::info;

use crate::pim::configv2::{ConfigV2, ConfigV3};

use super::{evil_filter::EvilFilter, translate_mapping::TranslateMapping};

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
    Ok(())
}

pub trait Simulator {
    type R;
    fn run(
        &mut self,
        mapping: impl TranslateMapping,
        matrix_tri_translated: &TriMatI<Pattern, u32>,
    ) -> eyre::Result<Self::R>;
}
