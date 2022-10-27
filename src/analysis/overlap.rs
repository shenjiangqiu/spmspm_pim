//! # the tools to compute the overlap between each tasks

use crate::pim::{
    config::Config,
    level::{ddr4, LevelTrait},
    task_manager::GraphATasks,
    SimulationContext,
};

#[derive(Debug)]
pub struct SingleTaskOverlapStat {
    overlap_histogram: Vec<usize>,
}

impl SingleTaskOverlapStat {
    pub fn new(max: usize) -> Self {
        Self {
            overlap_histogram: vec![0; max + 1],
        }
    }
    pub fn add(&mut self, overlap: usize) {
        let length = self.overlap_histogram.len();
        if overlap >= length {
            self.overlap_histogram[length - 1] += 1;
        } else {
            self.overlap_histogram[overlap] += 1;
        }
    }

    pub fn print(&self) {
        self.overlap_histogram
            .iter()
            .enumerate()
            .filter(|(_i, v)| **v != 0)
            .for_each(|(i, v)| {
                println!("overlap {} : {}", i, v);
            });
    }
}

pub fn compute_single_task_overlap_stat(config: &Config) -> SingleTaskOverlapStat {
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
            compute_single_task_overlap_stat_inner::<ddr4::Level>(config, &total_size)
        }
        crate::pim::config::DramType::LPDDR3 => todo!(),
        crate::pim::config::DramType::LPDDR4 => todo!(),
        crate::pim::config::DramType::HBM => todo!(),
        crate::pim::config::DramType::HBM2 => todo!(),
    }
}

fn compute_single_task_overlap_stat_inner<LevelType: LevelTrait>(
    config: &Config,
    total_size: &LevelType::Storage,
) -> SingleTaskOverlapStat
where
    LevelType::Storage: Ord,
{
    let mut stat = SingleTaskOverlapStat::new(1024);
    let graph_path = &config.graph_path;
    let graph_a = sprs::io::read_matrix_market(graph_path).unwrap().to_csr();
    let graph_b = graph_a.transpose_view().to_csr();
    let graph_b_mapping = LevelType::get_mapping(total_size, &graph_b);

    let mut context = SimulationContext::new(config);
    let graph_a_tasks: GraphATasks<LevelType> =
        GraphATasks::generate_mappings_for_a(&graph_a, &graph_b_mapping, &mut context);

    println!("graph_a_tasks: {:?}", graph_a_tasks.tasks.len());
    for target_task in graph_a_tasks.tasks.iter() {
        let num_rounds = target_task.tasks.len();
        stat.add(num_rounds);
    }
    stat
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_overlap() {
        let config: Config =
            toml::from_str(std::fs::read_to_string("ddr4.toml").unwrap().as_str()).unwrap();
        println!("{:?}", config);
        let stat = compute_single_task_overlap_stat(&config);
        println!("{:?}", stat);
    }
}
