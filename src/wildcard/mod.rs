use std::str::Chars;

// Finds whether the text matches/satisfies the pattern string.
// supports only '*' wildcard in the pattern.
// considers a file system path as a flat name space.
pub fn match_wildcard_simple(pattern: &str, name: &str) -> bool {
    if pattern.is_empty() {
        return name == pattern;
    }
    if pattern == "*" {
        return true;
    }
    // Does only wildcard '*' match.
    deep_match_char(
        &name.chars().collect::<Vec<char>>(),
        &pattern.chars().collect::<Vec<char>>(),
        true,
    )
}

// Finds whether the text matches/satisfies the pattern string.
// supports '*' and '?' wildcard in the pattern.
// considers a file system path as a flat name space.
pub fn match_wildcard(pattern: &str, name: &str) -> bool {
    if pattern.is_empty() {
        return name == pattern;
    }
    if pattern == "*" {
        return true;
    }
    // Does extended wildcard '*' and '?' match.
    deep_match_char(
        &name.chars().collect::<Vec<char>>(),
        &pattern.chars().collect::<Vec<char>>(),
        false,
    )
}

fn deep_match_char(mut name: &[char], mut pattern: &[char], simple: bool) -> bool {
    while !pattern.is_empty() {
        match pattern[0] {
            '?' => {
                if name.is_empty() && !simple {
                    return false;
                }
            }
            '*' => {
                return deep_match_char(name, &pattern[1..], simple)
                    || (!name.is_empty() && deep_match_char(&name[1..], pattern, simple));
            }
            _ => {
                if name.is_empty() || name[0] != pattern[0] {
                    return false;
                }
            }
        }
        name = &name[1..];
        pattern = &pattern[1..];
    }
    name.is_empty() && pattern.is_empty()
}
