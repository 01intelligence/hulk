use crate::ellipses;
use crate::errors::TypedError;
use anyhow::ensure;

// Supported set sizes this is used to find the optimal
// single set size.
const SET_SIZES: [usize; 13] = [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

// Checks whether given count is a valid set size for erasure coding.
fn is_valid_set_size(count: usize) -> bool {
    count >= SET_SIZES[0] && count <= SET_SIZES[SET_SIZES.len() - 1]
}

fn get_set_indexes(args: &[&str], total_sizes: &[usize], custom_set_drive_count: usize, arg_patterns: &[ellipses::ArgPattern]) -> anyhow::Result<Vec<Vec<usize>>> {
    ensure!(!total_sizes.is_empty() && !args.is_empty(), TypedError::InvalidArgument);

    for &total_size in total_sizes {
        ensure!(total_size >= SET_SIZES[0] && total_size >= custom_set_drive_count)
    }
    let set_indexes = total_sizes.iter();

    Ok(Default::default())
}
