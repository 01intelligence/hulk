use std::collections::HashMap;
use std::fmt;

use lazy_static::lazy_static;
use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, Serializer};

use super::*;
use crate::bucket::policy::Valid;

// Condition function trait.
pub trait Function: fmt::Display {
    // Evaluates this condition function with given values.
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool;

    // Returns condition key used in this function.
    fn key(&self) -> Key;

    // Returns condition name of this function.
    fn name(&self) -> Name;

    // Returns map representation of this function.
    fn to_map(&self) -> HashMap<Key, ValueSet>;
}

// List of functions.
pub struct Functions(Vec<Box<dyn Function>>);

impl Functions {
    pub fn new(functions: Vec<Box<dyn Function>>) -> Functions {
        Functions(functions)
    }

    // Evaluates all functions with given values map. Each function is evaluated
    // sequentially and next function is called only if current function succeeds.
    pub fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        self.0.iter().all(|f| f.evaluate(values))
    }

    // Returns list of keys used in all functions.
    pub fn keys(&self) -> KeySet {
        self.0.iter().map(|f| f.key()).collect()
    }
}

impl PartialEq for Functions {
    // Returns true if two Functions structures are equal.
    fn eq(&self, other: &Self) -> bool {
        self.0
            .iter()
            .all(|f| other.0.iter().any(|g| g.to_string() == f.to_string()))
    }
}

impl std::fmt::Display for Functions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut func_strings: Vec<String> = self.0.iter().map(|f| f.to_string()).collect();
        func_strings.sort_unstable();
        write!(f, "{:?}", func_strings)
    }
}

impl Serialize for Functions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut nm = HashMap::<Name, HashMap<Key, ValueSet>>::new();
        for f in &self.0 {
            let v = nm.entry(f.name()).or_default();
            v.extend(f.to_map());
        }
        let mut map = serializer.serialize_map(Some(nm.len()))?;
        for (k, v) in nm {
            map.serialize_entry(&k, &v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Functions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FunctionsVisitor;
        impl<'de> Visitor<'de> for FunctionsVisitor {
            type Value = Functions;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a condition value set")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                use serde::de::Error;
                let mut nm = HashMap::<Name, HashMap<Key, ValueSet>>::new();
                while let Some((k, v)) = map.next_entry()? {
                    nm.insert(k, v);
                }
                if nm.is_empty() {
                    return Err(A::Error::custom("condition must not be empty"));
                }
                let mut funcs = Functions::new(vec![]);
                for (name, args) in nm {
                    if !name.is_valid() {
                        return Err(A::Error::custom(format!(
                            "invalid condition name '{}'",
                            name
                        )));
                    }
                    for (key, values) in args {
                        if !key.is_valid() {
                            return Err(A::Error::custom(format!(
                                "invalid condition key '{}'",
                                key
                            )));
                        }
                        let vfn = match CONDITION_FUNC_MAP.get(&name) {
                            None => {
                                return Err(A::Error::custom(format!(
                                    "condition {} is not handled",
                                    name
                                )));
                            }
                            Some(vfn) => vfn,
                        };
                        let f = vfn(key, values).map_err(|e| A::Error::custom(format!("{}", e)))?;
                        funcs.0.push(f);
                    }
                }
                Ok(funcs)
            }
        }

        deserializer.deserialize_map(FunctionsVisitor)
    }
}

type NewFunction = fn(Key, ValueSet) -> anyhow::Result<Box<dyn Function + '_>>;

lazy_static! {
    static ref CONDITION_FUNC_MAP: HashMap<Name<'static>, NewFunction> = maplit::hashmap! {
        STRING_EQUALS => new_boolean_func as NewFunction,
    };
}
