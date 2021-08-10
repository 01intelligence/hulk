use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use anyhow::ensure;
use lazy_static::lazy_static;

use super::*;
use crate::ellipses;
use crate::errors::{TypedError, UiError};
use crate::fs::get_info;
use crate::strset::StringSet;

// Supported set sizes this is used to find the optimal
// single set size.
const SET_SIZES: [usize; 13] = [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

const ENV_ERASURE_SET_DRIVE_COUNT: &str = "HULK_ERASURE_SET_DRIVE_COUNT";

lazy_static! {
    pub static ref GLOBAL_CUSTOM_ERASURE_DRIVE_COUNT: Arc<Mutex<bool>> =
        Arc::new(Mutex::new(false));
}

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

fn common_set_drive_count(divisible_size: usize, set_counts: &[usize]) -> usize {
    if divisible_size < *set_counts.last().unwrap() {
        return divisible_size;
    }
    let mut set_size = 0;
    let mut prev_d = divisible_size / set_counts[0];
    for &cnt in set_counts {
        if divisible_size % cnt == 0 {
            let d = divisible_size / cnt;
            if d <= prev_d {
                prev_d = d;
                set_size = cnt;
            }
        }
    }
    set_size
}

fn possible_set_counts_with_symmetry(
    set_counts: &[usize],
    arg_patterns: &[ellipses::ArgPattern],
) -> Vec<usize> {
    let mut new_set_counts = HashSet::new();
    for &ss in set_counts {
        let mut symmetry = false;
        for arg_pattern in arg_patterns {
            for p in arg_pattern.iter() {
                if p.seq.len() > ss {
                    symmetry = p.seq.len() % ss == 0;
                } else {
                    symmetry = ss % p.seq.len() == 0;
                }
            }
        }
        if !new_set_counts.contains(&ss) && (symmetry || arg_patterns.is_empty()) {
            new_set_counts.insert(ss);
        }
    }

    let mut set_counts: Vec<usize> = new_set_counts.into_iter().collect();
    set_counts.sort_unstable();

    set_counts
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
            UiError::InvalidNumberOfErasureEndpoints
                .msg(format!("incorrect number of endpoints provided {:?}", args))
        );
    }

    let common_size = get_divisible_size(total_sizes);
    let set_counts: Vec<usize> = SET_SIZES
        .iter()
        .cloned()
        .filter(|&s| common_size % s == 0)
        .collect();
    ensure!(!set_counts.is_empty(), UiError::InvalidNumberOfErasureEndpoints.msg(format!("Incorrect number of endpoints provided {:?}, number of disks {} is not divisible by any supported erasure set sizes {:?}", args, common_size, SET_SIZES)));

    let set_size: usize;
    if custom_set_drive_count > 0 {
        let found = set_counts.iter().any(|&c| c == custom_set_drive_count);
        ensure!(
            found,
            UiError::InvalidErasureSetSize.msg(format!(
                "Invalid set drive count. Acceptable values for {} number drives are {:?}",
                common_size, set_counts
            ))
        );

        set_size = custom_set_drive_count;
        *GLOBAL_CUSTOM_ERASURE_DRIVE_COUNT.lock().unwrap() = true;
    } else {
        let set_counts = possible_set_counts_with_symmetry(&set_counts, arg_patterns);
        ensure!(!set_counts.is_empty(), UiError::InvalidNumberOfErasureEndpoints.msg(format!("No symmetric distribution detected with input endpoints provided {:?}, disks {} cannot be spread symmetrically by any supported erasure set sizes {:?}", args, common_size, SET_SIZES)));

        set_size = common_set_drive_count(common_size, &set_counts);
    }

    ensure!(is_valid_set_size(set_size), UiError::InvalidNumberOfErasureEndpoints.msg(format!("Incorrect number of endpoints provided {:?}, number of disks {} is not divisible by any supported erasure set sizes {:?}", args, common_size, SET_SIZES)));

    let set_indexes = total_sizes
        .iter()
        .map(|&total_size| {
            (0..total_size / set_size)
                .into_iter()
                .map(|_| set_size)
                .collect()
        })
        .collect();
    Ok(set_indexes)
}

fn get_total_sizes(arg_patterns: &[ellipses::ArgPattern]) -> Vec<usize> {
    arg_patterns
        .iter()
        .map(|arg_pattern| {
            arg_pattern
                .iter()
                .fold(1, |total_size, p| total_size * p.seq.len())
        })
        .collect()
}

#[derive(Default)]
struct EndpointSet {
    arg_patterns: Vec<ellipses::ArgPattern>,
    endpoints: Vec<String>,
    set_indexes: Vec<Vec<usize>>,
}

impl EndpointSet {
    fn get_endpoints(&mut self) {
        if self.endpoints.is_empty() {
            self.endpoints = self
                .arg_patterns
                .iter()
                .flat_map(|arg_pattern| arg_pattern.expand().into_iter())
                .map(|lbls| lbls.join(""))
                .collect();
        }
    }

    fn get(&mut self) -> Vec<Vec<String>> {
        self.get_endpoints();
        let mut k = 0;
        let mut sets = Vec::new();
        for s in &self.set_indexes {
            for &v in s {
                let s = &self.endpoints[k..v + k];
                sets.push(s.iter().cloned().collect());
                k += v;
            }
        }
        sets
    }
}

fn parse_endpoint_set<'a>(
    custom_set_drive_count: usize,
    args: &[&str],
) -> anyhow::Result<EndpointSet> {
    let mut arg_patterns = Vec::with_capacity(args.len());
    for &arg in args {
        arg_patterns.push(
            ellipses::find_ellipses_patterns(arg)
                .map_err(|e| UiError::InvalidErasureEndpoints.msg(e.to_string()))?,
        );
    }
    let set_indexes = get_set_indexes(
        args,
        &get_total_sizes(&arg_patterns),
        custom_set_drive_count,
        &arg_patterns,
    )
    .map_err(|e| UiError::InvalidErasureEndpoints.msg(e.to_string()))?;
    Ok(EndpointSet {
        arg_patterns,
        set_indexes,
        ..Default::default()
    })
}

fn get_all_sets(args: &[&str]) -> anyhow::Result<Vec<Vec<String>>> {
    let mut custom_set_drive_count: usize = 0;
    if let Ok(v) = std::env::var(ENV_ERASURE_SET_DRIVE_COUNT) {
        custom_set_drive_count = v
            .parse::<usize>()
            .map_err(|e| UiError::InvalidErasureSetSize.msg(e.to_string()))?;
    }

    let mut s;
    if !ellipses::has_ellipses(args) {
        let set_indexes = if args.len() > 1 {
            get_set_indexes(args, &vec![args.len()], custom_set_drive_count, &Vec::new())?
        } else {
            vec![vec![args.len()]]
        };
        s = EndpointSet {
            endpoints: args.iter().cloned().map(String::from).collect(),
            set_indexes,
            ..Default::default()
        };
    } else {
        s = parse_endpoint_set(custom_set_drive_count, args)?;
    }
    let set_args = s.get();

    let mut unique_args = StringSet::new();
    for args in &set_args {
        for arg in args {
            ensure!(
                !unique_args.contains(arg),
                UiError::InvalidErasureEndpoints
                    .msg(format!("Input args {:?} has duplicate ellipses", args))
            );
            unique_args.add(arg.clone());
        }
    }

    Ok(set_args)
}

pub fn create_server_endpoints(
    server_addr: &str,
    args: &[&str],
) -> anyhow::Result<(EndpointServerPools, SetupType)> {
    ensure!(!args.is_empty(), TypedError::InvalidArgument);

    if !ellipses::has_ellipses(args) {
        let set_args = get_all_sets(args)?;
    }

    todo!()
}
