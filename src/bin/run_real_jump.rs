use std::{
    collections::BTreeMap,
    fs::File,
    io::BufWriter,
    sync::atomic::{AtomicUsize, Ordering},
};
mod common;
use rayon::prelude::*;
use spmspm_pim::{
    analysis::{remap_analyze::real_jump, translate_mapping::TranslateMapping},
    init_logger_info,
    pim::configv2::{ConfigV3, MappingType},
    tools::file_server,
};
use sprs::{num_kinds::Pattern, CsMatI, TriMatI};
use tracing::{info, span::EnteredSpan};
static RUNNING_TASKS: AtomicUsize = AtomicUsize::new(0);
static FINISHED_TASKS: AtomicUsize = AtomicUsize::new(0);
static TOTAL_TASKS: AtomicUsize = AtomicUsize::new(0);
fn main() -> eyre::Result<()> {
    init_logger_info();
    let config: ConfigV3 =
        toml::from_str(include_str!("../../configs/real_jump_same_bank-1-16.toml")).unwrap();
    let total_graphs = config.graph_path.len() * 2 * 3;
    TOTAL_TASKS.store(total_graphs, Ordering::SeqCst);

    let result: common::RealJumpResultMap = config
        .graph_path
        .clone()
        .into_par_iter()
        .map(|graph_path| run_with_graph_path(graph_path, &config))
        .collect();
    serde_json::to_writer(
        BufWriter::new(File::create("output/real_jump_sensitive.json")?),
        &result,
    )?;

    Ok(())
}

fn run_with_graph_path(
    graph_path: String,
    config: &ConfigV3,
) -> (String, BTreeMap<MappingType, real_jump::RealJumpResult>) {
    let graph_path_file_name = graph_path.split('/').last().unwrap();
    let _span = tracing::span!(tracing::Level::INFO, "", g = graph_path_file_name).entered();
    let matrix_tri: TriMatI<Pattern, u32> = sprs::io::read_matrix_market_from_bufread(
        &mut file_server::file_reader(&graph_path).unwrap(),
    )
    .unwrap();
    let matrix_csr: CsMatI<Pattern, u32> = matrix_tri.to_csr();
    let result: BTreeMap<_, _> = [MappingType::SameBank, MappingType::SameBankWeightedMapping]
        .into_par_iter()
        .map(|map| run_with_mapping(map, config, &matrix_tri, &matrix_csr, &_span))
        .collect();
    (graph_path, result)
}

fn run_with_mapping(
    map: MappingType,
    config: &ConfigV3,
    matrix_tri: &TriMatI<Pattern, u32>,
    matrix_csr: &CsMatI<Pattern, u32>,
    parent_span: &EnteredSpan,
) -> (MappingType, real_jump::RealJumpResult) {
    // first build the mapping for the graph
    let _span = tracing::span!(parent: parent_span, tracing::Level::INFO, "", m=?map).entered();
    let result = match map {
        MappingType::SameBank => {
            let (mapping, matrix_csr) =
                real_jump::build_same_bank_mapping(&config, matrix_tri, matrix_csr);
            run_with_mapping_sp(&config, &map, &mapping, &matrix_csr)
        }
        MappingType::SameBankWeightedMapping => {
            let (mapping, matrix_csr) =
                real_jump::build_weighted_mapping(&config, matrix_tri, matrix_csr);
            run_with_mapping_sp(&config, &map, &mapping, &matrix_csr)
        }
        _ => unreachable!(),
    };
    (map, result)
}

fn run_with_mapping_sp<T: TranslateMapping + Sync>(
    config: &ConfigV3,
    map: &MappingType,
    mapping: &T,
    matrix_csr: &CsMatI<Pattern, u32>,
) -> real_jump::RealJumpResult {
    let mut config = config.clone();
    config.mapping = map.clone();

    RUNNING_TASKS.fetch_add(1, Ordering::SeqCst);
    info!(
        "started;  {} tasks running",
        RUNNING_TASKS.load(Ordering::SeqCst)
    );
    let reuslt = real_jump::run_with_mapping(mapping, &config, matrix_csr).unwrap();
    RUNNING_TASKS.fetch_sub(1, Ordering::SeqCst);
    FINISHED_TASKS.fetch_add(1, Ordering::SeqCst);
    info!(
        "finished {}/{} tasks, {} tasks running",
        FINISHED_TASKS.load(Ordering::SeqCst),
        TOTAL_TASKS.load(Ordering::SeqCst),
        RUNNING_TASKS.load(Ordering::SeqCst)
    );
    reuslt
}
