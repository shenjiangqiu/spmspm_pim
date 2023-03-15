use clap::Parser;
use image::{ImageBuffer, Luma};
use spmspm_pim::cli::GenerateGraphCli;
use sprs::{num_kinds::Pattern, CsMat};
use statrs::statistics::{Data, OrderStatistics};

fn main() {
    // read matrx from mtx/gearbox/soc-twitter-2010.mtx
    let cli = GenerateGraphCli::parse();
    let max_size = cli.max_size.unwrap_or(1920);
    let graph_path = cli.graph;
    let matrix: CsMat<Pattern> = sprs::io::read_matrix_market(graph_path).unwrap().to_csr();
    let image_size = matrix.rows().min(max_size);

    let image = matrix_to_image(matrix, (image_size, image_size));
    let output_path = cli.output.unwrap_or("image.png".into());
    image.save(output_path).unwrap();
}
// This function takes a CsMat<T> matrix and returns an ImageBuffer<Luma<u8>> image
pub fn matrix_to_image<T>(
    matrix: CsMat<T>,
    image_size: (usize, usize),
) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    // Get the shape of the matrix
    let (rows, cols) = matrix.shape();

    // Create a new image buffer with the desired size
    let mut image = ImageBuffer::new(image_size.0 as u32, image_size.1 as u32);
    let mut counts = vec![vec![0usize; image_size.0]; image_size.1];

    // Loop over the non-zero elements of the matrix
    for (row, vec) in matrix.outer_iterator().enumerate() {
        for (col, _) in vec.iter() {
            // Map the matrix coordinates to the image coordinates
            let x = col * image_size.0 / cols;
            let y = row * image_size.1 / rows;
            counts[x][y] += 1;
        }
    }
    // make the value from 0 to 255
    let max_data = counts.iter().flatten().max().unwrap();
    let orded_data: Vec<_> = counts.iter().flatten().map(|x| *x as f64).collect();
    let mut orded_data = Data::new(orded_data);
    let q_95 = orded_data.quantile(0.95);
    let q_95 = if q_95 == 0.0 { *max_data as f64 } else { q_95 };

    for (x, row) in counts.iter().enumerate() {
        for (y, count) in row.iter().enumerate() {
            let value = if *count as f64 > q_95 {
                255
            } else {
                *count * 255 / q_95 as usize
            };
            image.put_pixel(x as u32, y as u32, Luma([255 - value as u8]));
        }
    }
    // Return the image buffer
    image
}
