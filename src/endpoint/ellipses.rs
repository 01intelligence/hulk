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

#[derive(Default, Debug, Eq, PartialEq)]
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

pub async fn create_server_endpoints(
    server_addr: &str,
    args: &[&str],
) -> anyhow::Result<(EndpointServerPools, SetupType)> {
    ensure!(!args.is_empty(), TypedError::InvalidArgument);

    let mut endpoint_server_pools = EndpointServerPools::default();
    if !ellipses::has_ellipses(args) {
        let set_args = get_all_sets(args)?;
        let (endpoint_list, new_setup_type) =
            create_endpoints(server_addr, false, &set_args).await?;
        endpoint_server_pools.add(PoolEndpoints {
            set_count: set_args.len(),
            drives_per_set: set_args[0].len(),
            endpoints: endpoint_list,
        })?;
        return Ok((endpoint_server_pools, new_setup_type));
    }

    let mut setup_type = SetupType::Unknown;
    let mut found_prev_local = false;
    for arg in args {
        let set_args = get_all_sets(&[arg])?;
        let (endpoint_list, got_setup_type) =
            create_endpoints(server_addr, found_prev_local, &set_args).await?;
        found_prev_local = endpoint_list.at_least_one_endpoiont_local();
        endpoint_server_pools.add(PoolEndpoints {
            set_count: set_args.len(),
            drives_per_set: set_args[0].len(),
            endpoints: endpoint_list,
        })?;
        if setup_type == SetupType::Unknown {
            setup_type = got_setup_type
        }
        if setup_type == SetupType::Erasure && got_setup_type == SetupType::DistributedErasure {
            setup_type = SetupType::DistributedErasure
        }
    }

    Ok((endpoint_server_pools, setup_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_divisible_size() {
        let cases = vec![
            (&[24, 32, 16][..], 8),
            (&[32, 8, 4], 4),
            (&[8, 8, 8], 8),
            (&[24], 24),
        ];

        for (i, (total_sizes, expected_gcd)) in cases.into_iter().enumerate() {
            let gcd = get_divisible_size(total_sizes);
            assert_eq!(gcd, expected_gcd, "test {}", i + 1)
        }
    }

    #[test]
    fn test_parse_endpoint_set() {
        let cases = vec![
            // Test 1: Invalid inputs.
            ("", None),
            // Test 2: No range specified.
            ("{...}", None),
            // Test 3: Invalid range.
            ("http://hulk{2...3}/export/set{1...0}", None),
            // Test 4: Range cannot be smaller than 4 minimum.
            ("/export{1..2}", None),
            // Test 5: Unsupported characters.
            ("/export/test{1...2O}", None),
            // Test 6: Valid inputs.
            (
                "{1...27}",
                Some(EndpointSet {
                    arg_patterns: vec![ellipses::ArgPattern(vec![ellipses::Pattern {
                        prefix: "".to_string(),
                        suffix: "".to_string(),
                        seq: get_sequences(1, 27, 0),
                    }])],
                    endpoints: vec![],
                    set_indexes: vec![vec![9, 9, 9]],
                }),
            ),
            // Test 7: Valid inputs.
            (
                "/export/set{1...64}",
                Some(EndpointSet {
                    arg_patterns: vec![ellipses::ArgPattern(vec![ellipses::Pattern {
                        prefix: "/export/set".to_string(),
                        suffix: "".to_string(),
                        seq: get_sequences(1, 64, 0),
                    }])],
                    endpoints: vec![],
                    set_indexes: vec![vec![16, 16, 16, 16]],
                }),
            ),
            // Test 8: Valid input for distributed setup.
            (
                "http://hulk{2...3}/export/set{1...64}",
                Some(EndpointSet {
                    arg_patterns: vec![ellipses::ArgPattern(vec![
                        ellipses::Pattern {
                            prefix: "".to_string(),
                            suffix: "".to_string(),
                            seq: get_sequences(1, 64, 0),
                        },
                        ellipses::Pattern {
                            prefix: "http://hulk".to_string(),
                            suffix: "/export/set".to_string(),
                            seq: get_sequences(2, 3, 0),
                        },
                    ])],
                    endpoints: vec![],
                    set_indexes: vec![vec![16, 16, 16, 16, 16, 16, 16, 16]],
                }),
            ),
            // Test 9: Supporting some advanced cases.
            (
                "http://hulk{1...64}.mydomain.net/data",
                Some(EndpointSet {
                    arg_patterns: vec![ellipses::ArgPattern(vec![ellipses::Pattern {
                        prefix: "http://hulk".to_string(),
                        suffix: ".mydomain.net/data".to_string(),
                        seq: get_sequences(1, 64, 0),
                    }])],
                    endpoints: vec![],
                    set_indexes: vec![vec![16, 16, 16, 16]],
                }),
            ),
            // Test 10: Supporting some advanced cases.
            (
                "http://rack{1...4}.mydomain.hulk{1...16}/data",
                Some(EndpointSet {
                    arg_patterns: vec![ellipses::ArgPattern(vec![
                        ellipses::Pattern {
                            prefix: "".to_string(),
                            suffix: "/data".to_string(),
                            seq: get_sequences(1, 16, 0),
                        },
                        ellipses::Pattern {
                            prefix: "http://rack".to_string(),
                            suffix: ".mydomain.hulk".to_string(),
                            seq: get_sequences(1, 4, 0),
                        },
                    ])],
                    endpoints: vec![],
                    set_indexes: vec![vec![16, 16, 16, 16]],
                }),
            ),
            // Test 11: Supporting kubernetes cases.
            (
                "http://hulk{0...15}.mydomain.net/data{0...1}",
                Some(EndpointSet {
                    arg_patterns: vec![ellipses::ArgPattern(vec![
                        ellipses::Pattern {
                            prefix: "".to_string(),
                            suffix: "".to_string(),
                            seq: get_sequences(0, 1, 0),
                        },
                        ellipses::Pattern {
                            prefix: "http://hulk".to_string(),
                            suffix: ".mydomain.net/data".to_string(),
                            seq: get_sequences(0, 15, 0),
                        },
                    ])],
                    endpoints: vec![],
                    set_indexes: vec![vec![16, 16]],
                }),
            ),
            // Test 12: No host regex, just disks.
            (
                "http://server1/data{1...32}",
                Some(EndpointSet {
                    arg_patterns: vec![ellipses::ArgPattern(vec![ellipses::Pattern {
                        prefix: "http://server1/data".to_string(),
                        suffix: "".to_string(),
                        seq: get_sequences(1, 32, 0),
                    }])],
                    endpoints: vec![],
                    set_indexes: vec![vec![16, 16]],
                }),
            ),
            // Test 13: No host regex, just disks with two position numerics.
            (
                "http://server1/data{01...32}",
                Some(EndpointSet {
                    arg_patterns: vec![ellipses::ArgPattern(vec![ellipses::Pattern {
                        prefix: "http://server1/data".to_string(),
                        suffix: "".to_string(),
                        seq: get_sequences(1, 32, 2),
                    }])],
                    endpoints: vec![],
                    set_indexes: vec![vec![16, 16]],
                }),
            ),
            //  Test 14: More than 2 ellipses are supported as well.
            (
                "http://hulk{2...3}/export/set{1...64}/test{1...2}",
                Some(EndpointSet {
                    arg_patterns: vec![ellipses::ArgPattern(vec![
                        ellipses::Pattern {
                            prefix: "".to_string(),
                            suffix: "".to_string(),
                            seq: get_sequences(1, 2, 0),
                        },
                        ellipses::Pattern {
                            prefix: "".to_string(),
                            suffix: "/test".to_string(),
                            seq: get_sequences(1, 64, 0),
                        },
                        ellipses::Pattern {
                            prefix: "http://hulk".to_string(),
                            suffix: "/export/set".to_string(),
                            seq: get_sequences(2, 3, 0),
                        },
                    ])],
                    endpoints: vec![],
                    set_indexes: vec![vec![
                        16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
                    ]],
                }),
            ),
            // Test 15: More than 1 ellipses per argument for standalone setup.
            (
                "/export{1...10}/disk{1...10}",
                Some(EndpointSet {
                    arg_patterns: vec![ellipses::ArgPattern(vec![
                        ellipses::Pattern {
                            prefix: "".to_string(),
                            suffix: "".to_string(),
                            seq: get_sequences(1, 10, 0),
                        },
                        ellipses::Pattern {
                            prefix: "/export".to_string(),
                            suffix: "/disk".to_string(),
                            seq: get_sequences(1, 10, 0),
                        },
                    ])],
                    endpoints: vec![],
                    set_indexes: vec![vec![10, 10, 10, 10, 10, 10, 10, 10, 10, 10]],
                }),
            ),
            // Test 16: IPv6 ellipses with hexadecimal expansion
            (
                "http://[2001:3984:3989::{1...a}]/disk{1...10}",
                Some(EndpointSet {
                    arg_patterns: vec![ellipses::ArgPattern(vec![
                        ellipses::Pattern {
                            prefix: "".to_string(),
                            suffix: "".to_string(),
                            seq: get_sequences(1, 10, 0),
                        },
                        ellipses::Pattern {
                            prefix: "http://[2001:3984:3989::".to_string(),
                            suffix: "]/disk".to_string(),
                            seq: get_hex_sequences(1, 10, 0),
                        },
                    ])],
                    endpoints: vec![],
                    set_indexes: vec![vec![10, 10, 10, 10, 10, 10, 10, 10, 10, 10]],
                }),
            ),
            // Test 17: IPv6 ellipses with hexadecimal expansion with 3 position numerics.
            (
                "http://[2001:3984:3989::{001...00a}]/disk{1...10}",
                Some(EndpointSet {
                    arg_patterns: vec![ellipses::ArgPattern(vec![
                        ellipses::Pattern {
                            prefix: "".to_string(),
                            suffix: "".to_string(),
                            seq: get_sequences(1, 10, 0),
                        },
                        ellipses::Pattern {
                            prefix: "http://[2001:3984:3989::".to_string(),
                            suffix: "]/disk".to_string(),
                            seq: get_hex_sequences(1, 10, 3),
                        },
                    ])],
                    endpoints: vec![],
                    set_indexes: vec![vec![10, 10, 10, 10, 10, 10, 10, 10, 10, 10]],
                }),
            ),
        ];

        for (i, (arg, expected_es)) in cases.into_iter().enumerate() {
            match parse_endpoint_set(0, &[arg][..]) {
                Err(err) => assert!(
                    expected_es.is_none(),
                    "test {}: expected success but failed instead: {}",
                    i + 1,
                    err.to_string(),
                ),
                Ok(es) => assert_eq!(es, expected_es.unwrap(), "test {}", i + 1),
            }
        }
    }

    fn get_sequences(start: isize, number: isize, paddinglen: usize) -> Vec<String> {
        (start..=number)
            .map(|i| format!("{:01$}", i, paddinglen))
            .collect()
    }

    fn get_hex_sequences(start: isize, number: isize, paddinglen: usize) -> Vec<String> {
        (start..=number)
            .map(|i| format!("{:01$x}", i, paddinglen))
            .collect()
    }

    #[test]
    fn test_get_set_indexes() {
        let cases = vec![
            // Invalid inputs.
            // Test 1
            (&["data{1...3}"][..], &[3][..], None),
            // Test 2
            (
                &["data/controller1/export{1...2}, data/controller2/export{1...4}, data/controller3/export{1...8}"],
                &[2, 4, 8],
                None,
            ),
            // Test 3
            (&["data{1...17}/export{1...52}"], &[14144], None),
            // Valid inputs.
            // Test 4
            (&["data{1...27}"], &[27], Some(vec![vec![9, 9, 9]])),
            // Test 5
            (
                &["http://host{1...3}/data{1...180}"],
                &[540],
                Some(vec![vec![15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
                               15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
                               15, 15]]),
            ),
            // Test 6
            (
                &["http://host{1...2}.rack{1...4}/data{1...180}"],
                &[1440],
                Some(vec![vec![16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
                               16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
                               16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
                               16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
                               16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
                               16, 16, 16, 16, 16]]),
            ),
            // Test 7
            (
                &["http://host{1...2}/data{1...180}"],
                &[360],
                Some(vec![vec![12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
                               12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12]]),
            ),
            // Test 8
            (
                &["data/controller1/export{1...4}, data/controller2/export{1...8}, data/controller3/export{1...12}"],
                &[4, 8, 12],
                Some(vec![vec![4], vec![4, 4], vec![4, 4, 4]]),
            ),
            // Test 9
            (
                &["data{1...64}"],
                &[64],
                Some(vec![vec![16, 16, 16, 16]]),
            ),
            // Test 10
            (
                &["data{1...24}"],
                &[24],
                Some(vec![vec![12, 12]]),
            ),
            // Test 11
            (
                &["data/controller{1...11}/export{1...8}"],
                &[88],
                Some(vec![vec![11, 11, 11, 11, 11, 11, 11, 11]]),
            ),
            // Test 12
            (
                &["data{1...4}"],
                &[4],
                Some(vec![vec![4]]),
            ),
            // Test 13
            (
                &["data/controller1/export{1...10}, data/controller2/export{1...10}, data/controller3/export{1...10}"],
                &[10, 10, 10],
                Some(vec![vec![10], vec![10], vec![10]]),
            ),
            // Test 14
            (
                &["data{1...16}/export{1...52}"],
                &[832],
                Some(vec![vec![16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
                               16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
                               16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
                               16]]),
            ),
        ];

        for (i, (args, total_sizes, expected_indexes)) in cases.into_iter().enumerate() {
            let arg_patterns: Vec<ellipses::ArgPattern> = args
                .iter()
                .map(|arg| ellipses::find_ellipses_patterns(arg).unwrap())
                .collect();
            match get_set_indexes(args, total_sizes, 0, &arg_patterns) {
                Err(err) => assert!(expected_indexes.is_none(), "test {}", i + 1),
                Ok(indexes) => assert_eq!(indexes, expected_indexes.unwrap(), "test {}", i + 1),
            }
        }
    }

    #[test]
    fn test_get_set_indexes_env_override() {
        let cases = vec![
            // Test 1
            (
                &["data{1...64}"][..],
                &[64][..],
                8,
                Some(vec![vec![8, 8, 8, 8, 8, 8, 8, 8]]),
            ),
            // Test 2
            (
                &["http://host{1...2}/data{1...180}"],
                &[360],
                15,
                Some(vec![vec![
                    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
                    15, 15, 15, 15,
                ]]),
            ),
            // Test 3
            (
                &["http://host{1...12}/data{1...12}"],
                &[144],
                12,
                Some(vec![vec![12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12]]),
            ),
            // Test 4
            (
                &["http://host{0...5}/data{1...28}"],
                &[168],
                12,
                Some(vec![vec![
                    12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
                ]]),
            ),
            // Test 5
            (
                &["http://host{1...11}/data{1...11}"],
                &[121],
                11,
                Some(vec![vec![11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11]]),
            ),
            // Test 6
            (&["http://host{0...5}/data{1...28}"], &[168], 10, None),
            // Test 7
            (&["data{1...60}"], &[], 8, None),
            // Test 8
            (&["data{1...64}"], &[], 64, None),
            // Test 9
            (&["data{1...64}"], &[], 2, None),
        ];

        for (i, (args, total_sizes, env_override, expected_indexes)) in
            cases.into_iter().enumerate()
        {
            let arg_patterns: Vec<ellipses::ArgPattern> = args
                .iter()
                .map(|arg| ellipses::find_ellipses_patterns(arg).unwrap())
                .collect();
            match get_set_indexes(args, total_sizes, env_override, &arg_patterns) {
                Err(err) => assert!(expected_indexes.is_none(), "test {}", i + 1),
                Ok(indexes) => assert_eq!(indexes, expected_indexes.unwrap(), "test {}", i + 1),
            }
        }
    }
}
