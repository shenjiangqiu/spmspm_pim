use std::path::PathBuf;

use rayon::prelude::*;
use spmspm_pim::{
    analysis::remap_analyze,
    init_logger_info,
    pim::configv2::{ConfigV3, MappingType},
};
fn main() -> eyre::Result<()> {
    init_logger_info();
    let config: ConfigV3 =
        toml::from_str(include_str!("../../configs/real_jump_same_bank-1-16.toml")).unwrap();
    [MappingType::SameBank, MappingType::SameBankWeightedMapping]
        .into_par_iter()
        .for_each(|map| {
            [16, 32, 64].into_par_iter().for_each(|gap| {
                let mut config = config.clone();
                config.mapping = map.clone();
                config.remap_gap = gap;
                config.output_path =
                    PathBuf::from(format!("output/real_jump_{:?}-1-{}.json", map, gap));
                remap_analyze::run_simulation(config).unwrap();
            });
        });
    Ok(())
}
