use anyhow::bail;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // Regex to extract ellipses syntax inputs.
    static ref regex_ellipses: Regex =
        Regex::new(r#"(.*)({[0-9a-z]*\.\.\.[0-9a-z]*})(.*)"#).unwrap();
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
}

impl Pattern {
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

    let seq = Vec::new();
    for i in start..=end {}
    Ok(seq)
}

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
