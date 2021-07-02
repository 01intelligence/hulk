use anyhow::ensure;

use crate::ellipses;
use crate::errors::{self, TypedError};

// Supported set sizes this is used to find the optimal
// single set size.
const SET_SIZES: [usize; 13] = [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

// Checks whether given count is a valid set size for erasure coding.
fn is_valid_set_size(count: usize) -> bool {
    count >= SET_SIZES[0] && count <= SET_SIZES[SET_SIZES.len() - 1]
}

fn gcd(mut x: usize, mut y: usize) -> usize {
    while y != 0 {
        (x, y) = (y, x % y);
    }
    x
}

fn get_divisible_size(total_sizes: &[usize]) -> usize {
    total_sizes
        .iter()
        .cloned()
        .reduce(|a, b| gcd(a, b))
        .unwrap()
}

fn get_set_indexes(
    args: &[&str],
    total_sizes: &[usize],
    custom_set_drive_count: usize,
    arg_patterns: &[ellipses::ArgPattern],
) -> anyhow::Result<Vec<Vec<usize>>> {
    ensure!(
        !total_sizes.is_empty() && !args.is_empty(),
        TypedError::InvalidArgument
    );

    for &total_size in total_sizes {
        ensure!(
            total_size >= SET_SIZES[0] && total_size >= custom_set_drive_count,
            errors::UiErrorInvalidNumberOfErasureEndpoints
                .msg(format!("incorrect number of endpoints provided {:?}", args))
        );
    }
    let set_indexes = total_sizes.iter();

    let common_size = get_divisible_size(total_sizes);
    let set_counts: Vec<usize> = SET_SIZES
        .iter()
        .cloned()
        .filter(|&s| common_size % s == 0)
        .collect();
    ensure!(!set_counts.is_empty(), errors::UiErrorInvalidNumberOfErasureEndpoints.msg(format!("Incorrect number of endpoints provided {:?}, number of disks {} is not divisible by any supported erasure set sizes {:?}", args, common_size, SET_SIZES)));

    todo!()
}
