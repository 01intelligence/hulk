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

#[cfg(test)]
mod tests {
    use super::*;

    struct Match {
        pattern: &'static str,
        text: &'static str,
        matched: bool,
    }

    #[test]
    fn test_match_wildcard() {
        let cases = [
            // Test case - 1.
            // Test case with pattern "*". Expected to match any text.
            Match {
                pattern: "*",
                text: "s3:GetObject",
                matched: true,
            },
            // Test case - 2.
            // Test case with empty pattern. This only matches empty string.
            Match {
                pattern: "",
                text: "s3:GetObject",
                matched: false,
            },
            // Test case - 3.
            // Test case with empty pattern. This only matches empty string.
            Match {
                pattern: "",
                text: "",
                matched: true,
            },
            // Test case - 4.
            // Test case with single "*" at the end.
            Match {
                pattern: "s3:*",
                text: "s3:ListMultipartUploadParts",
                matched: true,
            },
            // Test case - 5.
            // Test case with a no "*". In this case the pattern and text should be the same.
            Match {
                pattern: "s3:ListBucketMultipartUploads",
                text: "s3:ListBucket",
                matched: false,
            },
            // Test case - 6.
            // Test case with a no "*". In this case the pattern and text should be the same.
            Match {
                pattern: "s3:ListBucket",
                text: "s3:ListBucket",
                matched: true,
            },
            // Test case - 7.
            // Test case with a no "*". In this case the pattern and text should be the same.
            Match {
                pattern: "s3:ListBucketMultipartUploads",
                text: "s3:ListBucketMultipartUploads",
                matched: true,
            },
            // Test case - 8.
            // Test case with pattern containing key name with a prefix. Should accept the same text without a "*".
            Match {
                pattern: "my-bucket/oo*",
                text: "my-bucket/oo",
                matched: true,
            },
            // Test case - 9.
            // Test case with "*" at the end of the pattern.
            Match {
                pattern: "my-bucket/In*",
                text: "my-bucket/India/Karnataka/",
                matched: true,
            },
            // Test case - 10.
            // Test case with prefixes shuffled.
            // This should fail.
            Match {
                pattern: "my-bucket/In*",
                text: "my-bucket/Karnataka/India/",
                matched: false,
            },
            // Test case - 11.
            // Test case with text expanded to the wildcards in the pattern.
            Match {
                pattern: "my-bucket/In*/Ka*/Ban",
                text: "my-bucket/India/Karnataka/Ban",
                matched: true,
            },
            // Test case - 12.
            // Test case with the  keyname part is repeated as prefix several times.
            // This is valid.
            Match {
                pattern: "my-bucket/In*/Ka*/Ban",
                text: "my-bucket/India/Karnataka/Ban/Ban/Ban/Ban/Ban",
                matched: true,
            },
            // Test case - 13.
            // Test case to validate that `*` can be expanded into multiple prefixes.
            Match {
                pattern: "my-bucket/In*/Ka*/Ban",
                text: "my-bucket/India/Karnataka/Area1/Area2/Area3/Ban",
                matched: true,
            },
            // Test case - 14.
            // Test case to validate that `*` can be expanded into multiple prefixes.
            Match {
                pattern: "my-bucket/In*/Ka*/Ban",
                text: "my-bucket/India/State1/State2/Karnataka/Area1/Area2/Area3/Ban",
                matched: true,
            },
            // Test case - 15.
            // Test case where the keyname part of the pattern is expanded in the text.
            Match {
                pattern: "my-bucket/In*/Ka*/Ban",
                text: "my-bucket/India/Karnataka/Bangalore",
                matched: false,
            },
            // Test case - 16.
            // Test case with prefixes and wildcard expanded for all "*".
            Match {
                pattern: "my-bucket/In*/Ka*/Ban*",
                text: "my-bucket/India/Karnataka/Bangalore",
                matched: true,
            },
            // Test case - 17.
            // Test case with keyname part being a wildcard in the pattern.
            Match {
                pattern: "my-bucket/*",
                text: "my-bucket/India",
                matched: true,
            },
            // Test case - 18.
            Match {
                pattern: "my-bucket/oo*",
                text: "my-bucket/odo",
                matched: false,
            },
            // Test case with pattern containing wildcard '?'.
            // Test case - 19.
            // "my-bucket?/" matches "my-bucket1/", "my-bucket2/", "my-bucket3" etc...
            // doesn't match "mybucket/".
            Match {
                pattern: "my-bucket?/abc*",
                text: "mybucket/abc",
                matched: false,
            },
            // Test case - 20.
            Match {
                pattern: "my-bucket?/abc*",
                text: "my-bucket1/abc",
                matched: true,
            },
            // Test case - 21.
            Match {
                pattern: "my-?-bucket/abc*",
                text: "my--bucket/abc",
                matched: false,
            },
            // Test case - 22.
            Match {
                pattern: "my-?-bucket/abc*",
                text: "my-1-bucket/abc",
                matched: true,
            },
            // Test case - 23.
            Match {
                pattern: "my-?-bucket/abc*",
                text: "my-k-bucket/abc",
                matched: true,
            },
            // Test case - 24.
            Match {
                pattern: "my??bucket/abc*",
                text: "mybucket/abc",
                matched: false,
            },
            // Test case - 25.
            Match {
                pattern: "my??bucket/abc*",
                text: "my4abucket/abc",
                matched: true,
            },
            // Test case - 26.
            Match {
                pattern: "my-bucket?abc*",
                text: "my-bucket/abc",
                matched: true,
            },
            // Test case 27-28.
            // '?' matches '/' too. (works with s3).
            // This is because the namespace is considered flat.
            // "abc?efg" matches both "abcdefg" and "abc/efg".
            Match {
                pattern: "my-bucket/abc?efg",
                text: "my-bucket/abcdefg",
                matched: true,
            },
            Match {
                pattern: "my-bucket/abc?efg",
                text: "my-bucket/abc/efg",
                matched: true,
            },
            // Test case - 29.
            Match {
                pattern: "my-bucket/abc????",
                text: "my-bucket/abc",
                matched: false,
            },
            // Test case - 30.
            Match {
                pattern: "my-bucket/abc????",
                text: "my-bucket/abcde",
                matched: false,
            },
            // Test case - 31.
            Match {
                pattern: "my-bucket/abc????",
                text: "my-bucket/abcdefg",
                matched: true,
            },
            // Test case 32-34.
            // test case with no '*'.
            Match {
                pattern: "my-bucket/abc?",
                text: "my-bucket/abc",
                matched: false,
            },
            Match {
                pattern: "my-bucket/abc?",
                text: "my-bucket/abcd",
                matched: true,
            },
            Match {
                pattern: "my-bucket/abc?",
                text: "my-bucket/abcde",
                matched: false,
            },
            // Test case 35.
            Match {
                pattern: "my-bucket/mnop*?",
                text: "my-bucket/mnop",
                matched: false,
            },
            // Test case 36.
            Match {
                pattern: "my-bucket/mnop*?",
                text: "my-bucket/mnopqrst/mnopqr",
                matched: true,
            },
            // Test case 37.
            Match {
                pattern: "my-bucket/mnop*?",
                text: "my-bucket/mnopqrst/mnopqrs",
                matched: true,
            },
            // Test case 38.
            Match {
                pattern: "my-bucket/mnop*?",
                text: "my-bucket/mnop",
                matched: false,
            },
            // Test case 39.
            Match {
                pattern: "my-bucket/mnop*?",
                text: "my-bucket/mnopq",
                matched: true,
            },
            // Test case 40.
            Match {
                pattern: "my-bucket/mnop*?",
                text: "my-bucket/mnopqr",
                matched: true,
            },
            // Test case 41.
            Match {
                pattern: "my-bucket/mnop*?and",
                text: "my-bucket/mnopqand",
                matched: true,
            },
            // Test case 42.
            Match {
                pattern: "my-bucket/mnop*?and",
                text: "my-bucket/mnopand",
                matched: false,
            },
            // Test case 43.
            Match {
                pattern: "my-bucket/mnop*?and",
                text: "my-bucket/mnopqand",
                matched: true,
            },
            // Test case 44.
            Match {
                pattern: "my-bucket/mnop*?",
                text: "my-bucket/mn",
                matched: false,
            },
            // Test case 45.
            Match {
                pattern: "my-bucket/mnop*?",
                text: "my-bucket/mnopqrst/mnopqrs",
                matched: true,
            },
            // Test case 46.
            Match {
                pattern: "my-bucket/mnop*??",
                text: "my-bucket/mnopqrst",
                matched: true,
            },
            // Test case 47.
            Match {
                pattern: "my-bucket/mnop*qrst",
                text: "my-bucket/mnopabcdegqrst",
                matched: true,
            },
            // Test case 48.
            Match {
                pattern: "my-bucket/mnop*?and",
                text: "my-bucket/mnopqand",
                matched: true,
            },
            // Test case 49.
            Match {
                pattern: "my-bucket/mnop*?and",
                text: "my-bucket/mnopand",
                matched: false,
            },
            // Test case 50.
            Match {
                pattern: "my-bucket/mnop*?and?",
                text: "my-bucket/mnopqanda",
                matched: true,
            },
            // Test case 51.
            Match {
                pattern: "my-bucket/mnop*?and",
                text: "my-bucket/mnopqanda",
                matched: false,
            },
            // Test case 52.
            Match {
                pattern: "my-?-bucket/abc*",
                text: "my-bucket/mnopqanda",
                matched: false,
            },
        ];

        // Iterating over the test cases, call the function under test and asert the output.
        for (i, case) in cases.iter().enumerate() {
            let actual_result = match_wildcard(case.pattern, case.text);
            assert_eq!(
                case.matched,
                actual_result,
                "Test {}: Expected the result to be `{}`, but instead found it to be `{}`",
                i + 1,
                case.matched,
                actual_result
            )
        }
    }
}
