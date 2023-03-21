#[allow(unused)]
mod generate_matrix_graph;
use generate_matrix_graph::matrix_to_image;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use spmspm_pim::init_logger_stderr;
use spmspm_pim::pim::configv2;
use sprs::{num_kinds::Pattern, CsMat};
use std::path::Path;
use tracing::{info, metadata::LevelFilter};
fn main() {
    init_logger_stderr(LevelFilter::INFO);
    let config_v2 = configv2::ConfigV2::new("configs/gearbox_001_v2.toml");
    let graphs = config_v2.graph_path;
    graphs.into_par_iter().for_each(|e| {
        let path = Path::new(&e);
        info!("Processing {:?}", path);
        let output = Path::new("images").join(path.with_extension("png").file_name().unwrap());
        let graph: CsMat<Pattern> = sprs::io::read_matrix_market(path).unwrap().to_csr();
        let rows = graph.rows();
        let size = rows.min(1920);
        let image = matrix_to_image(graph, (size, size));
        std::fs::create_dir_all(output.parent().unwrap()).unwrap();
        image.save(output).unwrap();
    });
}
