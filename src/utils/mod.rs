pub fn ceil_frac(mut numerator: isize, mut denominator: isize) -> isize {
    if denominator == 0 {
        // do nothing on invalid input
        return 0;
    }
    // Make denominator positive
    if denominator < 0 {
        numerator = -numerator;
        denominator = -denominator;
    }
    let mut ceil = numerator / denominator;
    if numerator > 0 && numerator % denominator != 0 {
        ceil += 1;
    }
    ceil
}
