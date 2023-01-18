use crate::cli::DrawType;

pub fn draw_with_type(args: DrawType) -> eyre::Result<()> {
    match args {
        DrawType::SpeedUp(speed_up_args) => super::speedup::draw_speedup(speed_up_args)?,
        DrawType::Split(split_args) => super::draw_split::draw_split(split_args)?,
        DrawType::Empty(split_args) => super::empty::draw_empty(split_args)?,
        DrawType::Cycle(split_args) => super::cycle_dist::draw_cycle_dist(split_args)?,
        DrawType::Gearbox(gearbox_result) => super::gearbox::draw_gearbox(gearbox_result)?,
        DrawType::GearboxOld(gearbox_result) => {
            super::gearbox_old::draw_gearbox_old(gearbox_result)?
        }
        DrawType::GearBoxAll(gearbox_result) => {
            super::gearbox_all::draw_gearbox_all(gearbox_result)?
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use sprs::{num_kinds::Pattern, CsMat};

    #[test]
    fn test_read_mtx() {
        const MTX_PATH: &str = "mtx/gearbox/ca-hollywood-2009.mtx";
        let _graph: CsMat<Pattern> = sprs::io::read_matrix_market(MTX_PATH).unwrap().to_csr();
    }
}
