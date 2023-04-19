#[cfg(test)]
mod tests {
    use rayon::prelude::*;

    #[test]
    fn test() {
        let a = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let result = a
            .par_iter()
            .map(|x| x * x)
            .reduce(|| 0, |acc, item| acc.max(item));
        println!("result: {}", result);
    }
}
