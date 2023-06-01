use crate::analysis::remap_analyze::row_cycle::*;
use itertools::Itertools;
use sprs::{num_kinds::Pattern, CsMatI, TriMatI};
use tracing::debug;

use crate::tools;

pub struct SameBankMapping {
    row_sub_mapping: super::AverageMapping,
    evil_threshold: usize,
}

impl super::TranslateMapping for SameBankMapping {
    type RowSubMapping = super::AverageMapping;

    fn get_row_sub_mapping(&self) -> &Self::RowSubMapping {
        &self.row_sub_mapping
    }

    fn is_evil(&self, row_id: LogicRowId) -> bool {
        row_id.0 < self.evil_threshold
    }
}

impl SameBankMapping {
    pub fn new(
        total_banks: usize,
        total_channels: usize,
        total_subarrays: usize,
        evil_threshold: usize,
        cols: usize,
        graph: &TriMatI<Pattern, u32>,
        graph_csr: &CsMatI<Pattern, u32>,
    ) -> (Self, CsMatI<Pattern, u32>) {
        // step 1, select the evil rows
        let initial_mapping =
            super::evil_mapping::build_evil_row_mapping(graph_csr, evil_threshold);
        //step 2, create mapping for remaining rows
        let (initial_evil, initial_non_evil) = initial_mapping.split_at(evil_threshold);
        let non_evil_mapping = build_non_evil_mapping(
            total_channels,
            total_banks,
            total_subarrays,
            initial_non_evil,
        );
        let mapping = initial_evil
            .iter()
            .cloned()
            .chain(non_evil_mapping)
            .collect_vec();

        let translated_graph =
            tools::remapping_translate::translate(graph.view(), &mapping).to_csr();

        let row_sub_mapping = super::AverageMapping::new(
            translated_graph.view(),
            evil_threshold,
            total_subarrays * total_banks * total_channels,
            cols,
        );
        (
            Self {
                row_sub_mapping,
                evil_threshold,
            },
            translated_graph,
        )
    }
}

fn build_non_evil_mapping(
    total_channels: usize,
    total_banks: usize,
    total_subarrays: usize,
    initial_non_evil_mapping: &[usize],
) -> Vec<usize> {
    let mut non_evil_mapping = vec![];
    // first, distribute the rows to banks,
    // first calculate the average weight
    let global_total_banks = total_channels * total_banks;

    let graph_rows = initial_non_evil_mapping.len();
    let average_rows_per_bank = graph_rows / global_total_banks;
    let bank_rows = (0..global_total_banks).map(|i| {
        let (start, end) = if i != global_total_banks - 1 {
            let start = i * average_rows_per_bank;
            let end = (i + 1) * average_rows_per_bank;
            (start, end)
        } else {
            let start = i * average_rows_per_bank;
            let end = graph_rows;
            (start, end)
        };
        debug!("bank {} has rows {} to {}", i, start, end);
        (start, end)
    });

    // in this loop, will setup the row_id_mappings, which contains the detailed mappings for
    // each row!
    for (_bank_id, (start_row_id, end_row_id)) in bank_rows.into_iter().enumerate() {
        // this represent a bank, first create the subarray status for each subarray
        #[derive(Debug, Clone)]
        struct SubarrayStatus {
            rows: Vec<usize>,
        }
        let mut subarray_status = vec![SubarrayStatus { rows: vec![] }; total_subarrays];
        // for each row, put it into the subarray
        for (subarray_id, row_id) in (start_row_id..end_row_id)
            .enumerate()
            .map(|(index, row_id)| (index % total_subarrays, row_id))
        {
            let subarray = &mut subarray_status[subarray_id];

            subarray.rows.push(initial_non_evil_mapping[row_id]);
        }
        // update the mapping
        non_evil_mapping.extend(
            subarray_status
                .into_iter()
                .flat_map(|subarray| subarray.rows),
        );
    }
    non_evil_mapping
}

#[cfg(test)]
mod tests {
    use sprs::{num_kinds::Pattern, TriMatI};
    use tracing::metadata::LevelFilter;

    use crate::analysis::remap_analyze::row_cycle::*;
    use crate::{analysis::translate_mapping::TranslateMapping, init_logger_stderr};

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_dist() {
        init_logger_stderr(LevelFilter::DEBUG);
        let matrix: TriMatI<Pattern, u32> =
            sprs::io::read_matrix_market("test_mtx/bcspwr03.mtx").unwrap();
        let (mapping, translated_matrix) =
            super::SameBankMapping::new(1, 2, 2, 16, 16, &matrix, &matrix.to_csr());
        let translated_matrix_csr = translated_matrix.to_csr();
        // test the evil mapping
        for row in 0..16 {
            let loc =
                mapping.get_location_evil(LogicRowId(row as usize), translated_matrix_csr.view());
            println!("row: {}, loc: {:?}", row, loc);
        }
        // test the non evil mapping
        for row in 16..matrix.rows() {
            let loc = mapping.get_location(row.into(), translated_matrix_csr.view());
            println!(
                "row: {}, loc: {:?}, nnz: {}",
                row,
                loc,
                translated_matrix_csr.outer_view(row).unwrap().nnz()
            );
        }
        // test the dense mapping
        for col in 0..matrix.rows() {
            let loc =
                mapping.get_dense_location(0.into(), col.into(), translated_matrix_csr.view());
            println!("col: {}, loc: {:?}", col, loc,);
        }
    }
}
