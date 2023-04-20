#[cfg(test)]
mod tests {
    use rayon::prelude::*;
    use tracing::{info, metadata::LevelFilter, Level};

    #[test]
    fn test() {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            )
            .init();
        let a = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        a.par_iter().for_each(|x| {
            let _span = tracing::span!(Level::INFO, "LEVEL1", x).entered();
            a.par_iter().for_each(|y| {
                let _span = tracing::span!(parent: &_span, Level::INFO, "LEVEL2", y).entered();
                a.par_iter().for_each(|z| {
                    let _span = tracing::span!(parent: &_span, Level::INFO, "LEVEL3", z).entered();
                    info!("{} {} {}", x, y, z);
                });
            });
        });
    }
}
