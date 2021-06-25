use relative_path::{RelativePath, RelativePathBuf};

pub const SLASH_SEPARATOR: &str = "/";

// Join paths and retains trailing SlashSeparator of the last element.
pub fn path_join(elements: &[&str]) -> String {
    if elements.is_empty() {
        return "".to_owned();
    }
    let mut p = RelativePathBuf::new();
    for e in elements {
        p.push(e);
    }
    // Retail prefix slash.
    let mut s = if elements[0].starts_with(SLASH_SEPARATOR) {
        SLASH_SEPARATOR.to_owned() + &p.normalize().to_string()
    } else {
        p.normalize().to_string()
    };
    // Retail suffix slash.
    if elements[elements.len() - 1].ends_with(SLASH_SEPARATOR) {
        s.push_str(SLASH_SEPARATOR);
    }
    return s;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_join() {
        let cases = vec![
            (vec![], ""),
            (vec![""], ""),
            (vec!["a"], "a"),
            (vec!["a", "b"], "a/b"),
            (vec!["a", ""], "a"),
            (vec!["", "b"], "b"),
            (vec!["/", "a"], "/a"),
            (vec!["/", ""], "/"),
            (vec!["a/", "b"], "a/b"),
            (vec!["a/", ""], "a"),
            (vec!["", ""], ""),
            (vec!["a", "b/"], "a/b/"),
        ];
        for (elements, path) in cases {
            assert_eq!(path_join(&elements), path);
        }
    }
}
