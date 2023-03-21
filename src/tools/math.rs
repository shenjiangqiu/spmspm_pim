/// calculate the how many bits are needed to represent the count(0->count-1),
/// like 8 to 3, 16 to 4
/// ```
/// use spmspm_pim::tools::math::count_to_log;
/// assert_eq!(count_to_log(8), 3);
/// assert_eq!(count_to_log(16), 4);
/// assert_eq!(count_to_log(32), 5);
/// assert_eq!(count_to_log(64), 6);
/// ```
///
/// when count is not power of 2, it will be rounded up to the next power of 2
///
///
/// ```
/// use spmspm_pim::tools::math::count_to_log;
/// assert_eq!(count_to_log(9), 4);
/// assert_eq!(count_to_log(17), 5);
/// ```
///
///
pub fn count_to_log(count: usize) -> usize {
    // like 8 to 3, 16 to 4
    if count == 0 {
        return 0;
    }
    let count = count - 1;
    // how many bits it has
    let mut bits = 0;
    let mut count = count;
    while count > 0 {
        count >>= 1;
        bits += 1;
    }
    bits
}
