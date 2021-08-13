use std::fmt;

use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::Serialize;

use crate::utils::assert::assert_ok;

#[derive(Debug, PartialEq, Serialize)]
pub enum BoolFlag {
    #[serde(rename(serialize = "on"))]
    On,
    #[serde(rename(serialize = "off"))]
    Off,
}

impl Default for BoolFlag {
    fn default() -> Self {
        BoolFlag::Off
    }
}

impl fmt::Display for BoolFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            BoolFlag::On => "on",
            BoolFlag::Off => "off",
        };
        write!(f, "{}", s)
    }
}

impl<'de> Deserialize<'de> for BoolFlag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BoolFlagVisitor;
        impl<'de> Visitor<'de> for BoolFlagVisitor {
            type Value = BoolFlag;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a bool flag")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v.len() == 0 || v == "" {
                    // Empty string is treated as valid.
                    return Ok(BoolFlag::On);
                }
                let result_flag = crate::utils::parse_bool_ext(v);
                match result_flag {
                    Ok(flag) => {
                        if flag {
                            Ok(BoolFlag::On)
                        } else {
                            Ok(BoolFlag::Off)
                        }
                    }
                    Err(err) => Err(E::custom(err)),
                }
            }
        }
        deserializer.deserialize_str(BoolFlagVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_flag_string() {
        let cases: [(BoolFlag, &str); 3] = [
            (BoolFlag::default(), "off"),
            (BoolFlag::On, "on"),
            (BoolFlag::Off, "off"),
        ];
        for (flag, expected_result) in cases.iter() {
            let result_string = flag.to_string();
            let result_format = format!("{}", flag);
            assert_eq!(result_string, *expected_result);
            assert_eq!(result_format, *expected_result);
        }
    }

    #[test]
    fn test_bool_flag_serialize() {
        let cases: [(BoolFlag, &str); 3] = [
            (BoolFlag::default(), r#""off""#),
            (BoolFlag::On, r#""on""#),
            (BoolFlag::Off, r#""off""#),
        ];
        for (flag, expected_result) in cases.iter() {
            let flag = assert_ok!(serde_json::to_string(flag));
            assert_eq!(flag, *expected_result);
        }
    }

    #[test]
    fn test_bool_flag_deserialize() {
        let cases: [(&str, BoolFlag, bool); 10] = [
            (r#"{}"#, BoolFlag::Off, true),
            (r#"["on"]"#, BoolFlag::Off, true),
            (r#""junk""#, BoolFlag::Off, true),
            (r#""""#, BoolFlag::On, false),
            (r#""on""#, BoolFlag::On, false),
            (r#""off""#, BoolFlag::Off, false),
            (r#""true""#, BoolFlag::On, false),
            (r#""false""#, BoolFlag::Off, false),
            (r#""ON""#, BoolFlag::On, false),
            (r#""OFF""#, BoolFlag::Off, false),
        ];
        for (data, expected_result, expected_err) in cases.iter() {
            let result: Result<BoolFlag, serde_json::Error> = serde_json::from_str(data);
            match result {
                Ok(flag) => assert_eq!(flag, *expected_result),
                Err(_) => assert!(expected_err),
            }
        }
    }
}
