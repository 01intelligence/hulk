use anyhow::{anyhow, bail};
use lazy_static::lazy_static;
use regex::Regex;
use std::slice::Iter;

lazy_static! {
    // Regex to extract ellipses syntax inputs.
    static ref REGEX_ELLIPSES: Regex =
        Regex::new(r#"(.*)(\{[0-9a-z]*\.\.\.[0-9a-z]*\})(.*)"#).unwrap();
}

// Ellipses constants
const OPEN_BRACES: char = '{';
const CLOSE_BRACES: char = '}';
const ELLIPSES: &str = "...";

// Ellipses pattern, describes the range and also the
// associated prefix and suffixes.
pub struct Pattern {
    pub prefix: String,
    pub suffix: String,
    pub seq: Vec<String>,
}

// A list of patterns provided in the input.
pub struct ArgPattern(Vec<Pattern>);

impl ArgPattern {
    // Expands all the ellipses patterns in the given argument.
    pub fn expand(&self) -> Vec<Vec<String>> {
        let mut labels = Vec::with_capacity(self.0.len());
        for v in &self.0 {
            labels.push(v.expand());
        }
        arg_expander(&labels)
    }

    pub fn iter(&self) -> Iter<'_, Pattern> {
        self.0.iter()
    }
}

impl Pattern {
    // Expands a ellipses pattern.
    pub fn expand(&self) -> Vec<String> {
        let mut labels = Vec::with_capacity(self.seq.len());
        for s in &self.seq {
            if !self.prefix.is_empty() && self.suffix.is_empty() {
                labels.push(format!("{}{}", self.prefix, s));
            } else if self.prefix.is_empty() && !self.suffix.is_empty() {
                labels.push(format!("{}{}", s, self.suffix));
            } else if self.prefix.is_empty() && self.suffix.is_empty() {
                labels.push(s.to_owned());
            } else {
                labels.push(format!("{}{}{}", self.prefix, s, self.suffix));
            }
        }
        labels
    }
}

// Parses an ellipses range pattern of following style
// `{1...64}`
// `{33...64}`
fn parse_ellipses_range(mut pattern: &str) -> anyhow::Result<Vec<String>> {
    if !pattern.contains(OPEN_BRACES) || !pattern.contains(CLOSE_BRACES) {
        bail!("invalid argument");
    }
    pattern = pattern.trim_matches(|c| c == OPEN_BRACES || c == CLOSE_BRACES);
    let ellipses_range: Vec<&str> = pattern.split(ELLIPSES).collect();
    if ellipses_range.len() != 2 {
        bail!("invalid argument");
    }
    let mut hexadecimal = false;
    let start = if let Ok(start) = ellipses_range[0].parse::<u64>() {
        start
    } else {
        // Look for hexadecimal conversions if any.
        hexadecimal = true;
        u64::from_str_radix(ellipses_range[0], 16)?
    };
    let end = if let Ok(end) = ellipses_range[1].parse::<u64>() {
        end
    } else {
        // Look for hexadecimal conversions if any.
        hexadecimal = true;
        u64::from_str_radix(ellipses_range[1], 16)?
    };
    if start > end {
        bail!(
            "Incorrect range start {} cannot be bigger than end {}",
            start,
            end
        )
    }

    let mut seq = Vec::new();
    for i in start..=end {
        if ellipses_range[0].starts_with('0') && ellipses_range[0].len() > 1
            || ellipses_range[1].starts_with('0')
        {
            if hexadecimal {
                seq.push(format!("{:0width$x}", i, width = ellipses_range[1].len()));
            } else {
                seq.push(format!("{:0width$}", i, width = ellipses_range[1].len()));
            }
        } else {
            if hexadecimal {
                seq.push(format!("{:x}", i));
            } else {
                seq.push(format!("{}", i));
            }
        }
    }
    Ok(seq)
}

// Recursively expands labels into its respective forms.
fn arg_expander(labels: &[Vec<String>]) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    if labels.len() == 1 {
        for v in &labels[0] {
            out.push(vec![v.to_owned()]);
        }
        return out;
    }
    for lbl in &labels[0] {
        for mut r in arg_expander(&labels[1..]) {
            r.extend(vec![lbl.to_owned()]);
            out.push(r);
        }
    }
    out
}

// Returns true if input arg has ellipses type pattern.
pub fn has_ellipses(args: &[&str]) -> bool {
    args.iter()
        .all(|&a| a.contains(ELLIPSES) || (a.contains(OPEN_BRACES) && a.contains(CLOSE_BRACES)))
}

fn err_invalid_ellipses_format_fn(arg: &str) -> anyhow::Error {
    anyhow!("Invalid ellipsis format in '{}', Ellipsis range must be provided in format {{N...M}} where N and M are positive integers, M must be greater than N,  with an allowed minimum range of 4", arg)
}

// Finds all ellipses patterns, recursively and parses the ranges numerically.
pub fn find_ellipses_patterns(arg: &str) -> anyhow::Result<ArgPattern> {
    // We throw an error if arg doesn't have any recognizable ellipses pattern.
    let mut parts: Vec<&str> = REGEX_ELLIPSES
        .captures(arg)
        .ok_or(err_invalid_ellipses_format_fn(arg))?
        .iter()
        .map(|v| v.unwrap().as_str())
        .collect();
    parts = parts.into_iter().skip(1).collect();

    let mut patterns: Vec<Pattern> = Vec::new();
    let mut pattern_found = REGEX_ELLIPSES.is_match(parts[0]);
    while pattern_found {
        let seq = parse_ellipses_range(parts[1])?;
        patterns.push(Pattern {
            prefix: "".to_owned(),
            suffix: parts[2].to_owned(),
            seq,
        });
        if let Some(mut p) = REGEX_ELLIPSES.captures(parts[0]) {
            parts = p.iter().map(|v| v.unwrap().as_str()).collect();
            parts = parts.into_iter().skip(1).collect();
            pattern_found = has_ellipses(&vec![parts[0]]);
            continue;
        }
        break;
    }

    if parts.len() > 0 {
        let seq = parse_ellipses_range(parts[1])?;
        patterns.push(Pattern {
            prefix: parts[0].to_owned(),
            suffix: parts[2].to_owned(),
            seq,
        })
    }

    // Check if any of the prefix or suffixes now have flower braces
    // left over, in such a case we generally think that there is
    // perhaps a typo in users input and error out accordingly.
    for pattern in &patterns {
        if pattern.prefix.contains(OPEN_BRACES)
            || pattern.prefix.contains(CLOSE_BRACES)
            || pattern.suffix.contains(OPEN_BRACES)
            || pattern.suffix.contains(CLOSE_BRACES)
        {
            return Err(err_invalid_ellipses_format_fn(arg));
        }
    }

    Ok(ArgPattern(patterns))
}
