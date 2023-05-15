use std::collections::BTreeMap;

use sprs::{num_kinds::Pattern, TriMatI, TriMatViewI};

pub fn translate(
    input_matrix: TriMatViewI<Pattern, u32>,
    mapping: &[usize],
) -> TriMatI<Pattern, u32> {
    // the mapping is  new_to_old, should be the inverse of the old_to_new
    let mapping: BTreeMap<usize, usize> =
        mapping.iter().enumerate().map(|(i, v)| (*v, i)).collect();
    let mut output_matrix = TriMatI::new((input_matrix.rows(), input_matrix.cols()));
    for (v, (i, j)) in input_matrix.into_iter() {
        output_matrix.add_triplet(mapping[&(i as usize)], mapping[&(j as usize)], *v);
    }
    output_matrix
}

#[cfg(test)]
mod tests {
    use sprs::num_kinds::Pattern;

    use super::translate;

    #[test]
    fn test_simple_translate() {
        let mut input_matrix = sprs::TriMatI::new((3, 3));
        input_matrix.add_triplet(0, 1, Pattern);
        input_matrix.add_triplet(1, 2, Pattern);
        input_matrix.add_triplet(2, 0, Pattern);

        let output_matrix = translate(input_matrix.view(), &[1, 0, 2]);
        for (v, (i, j)) in output_matrix.into_iter() {
            println!("({}, {}) = {:?}", i, j, v);
        }
    }
}
