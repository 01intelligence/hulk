use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use anyhow::bail;
use ipnet::IpNet;

use super::super::Valid;
use super::*;

// IP address function. It checks whether value by Key in given
// values is in IP network.  Here Key must be AWSSourceIP.
// For example,
//   - if values = [192.168.1.0/24], at evaluate() it returns whether IP address
//     in value map for AWSSourceIP falls in the network 192.168.1.10/24.
#[derive(Clone)]
pub(super) struct IpAddressFunc<'a> {
    key: Key<'a>,
    values: Vec<IpNet>,
}

impl<'a> fmt::Display for IpAddressFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:", IP_ADDRESS, self.key)?;
        write!(f, "[")?;
        for v in &self.values {
            write!(f, "{}", v.to_string())?;
        }
        write!(f, "]")
    }
}

impl<'a> Function for IpAddressFunc<'a> {
    // Evaluates to check whether IP address in values map for AWSSourceIP
    // falls in one of network or not.
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        let mut v = values.get(&canonical_key(self.key.name()));
        if v.is_none() {
            v = values.get(self.key.name());
        }
        match v {
            Some(v) => {
                for s in v {
                    let ip = std::net::IpAddr::from_str(s).unwrap();
                    if self.values.iter().any(|net| net.contains(&ip)) {
                        return true;
                    }
                }
                return false;
            }
            None => false,
        }
    }

    fn key(&self) -> Key<'a> {
        self.key.clone()
    }

    fn name(&self) -> Name<'a> {
        IP_ADDRESS
    }

    fn to_map(&self) -> HashMap<Key<'a>, ValueSet> {
        let mut map = HashMap::new();
        if !self.key.is_valid() {
            return map;
        }
        let values = ValueSet::new(
            self.values
                .as_slice()
                .iter()
                .map(|&v| Value::String(v.to_string()))
                .collect(),
        );
        map.insert(self.key.clone(), values);
        map
    }
}

// Not IP address function. It checks whether value by Key in given
// values is NOT in IP network.  Here Key must be AWSSourceIP.
// For example,
//   - if values = [192.168.1.0/24], at evaluate() it returns whether IP address
//     in value map for AWSSourceIP does not fall in the network 192.168.1.10/24.
#[derive(Clone)]
pub(super) struct NotIpaddressFunc<'a>(IpAddressFunc<'a>);

impl<'a> fmt::Display for NotIpaddressFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:", NOT_IP_ADDRESS, self.0.key)?;
        write!(f, "[")?;
        for v in &self.0.values {
            write!(f, "{}", v.to_string())?;
        }
        write!(f, "]")
    }
}

impl<'a> Function for NotIpaddressFunc<'a> {
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        !self.0.evaluate(values)
    }

    fn key(&self) -> Key<'_> {
        self.0.key()
    }

    fn name(&self) -> Name<'_> {
        NOT_IP_ADDRESS
    }

    fn to_map(&self) -> HashMap<Key<'_>, ValueSet> {
        self.0.to_map()
    }
}

pub(crate) fn new_ip_address_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    if key != AWS_SOURCE_IP {
        bail!(
            "only '{}' key is allowed for {} condition",
            AWS_SOURCE_IP,
            IP_ADDRESS
        );
    }
    let ip_nets = values_to_ip_nets(IP_ADDRESS, &values)?;
    Ok(Box::new(IpAddressFunc {
        key,
        values: ip_nets,
    }))
}

pub(super) fn new_not_ip_address_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    if key != AWS_SOURCE_IP {
        bail!(
            "only '{}' key is allowed for {} condition",
            AWS_SOURCE_IP,
            NOT_IP_ADDRESS
        );
    }
    let ip_nets = values_to_ip_nets(NOT_IP_ADDRESS, &values)?;
    Ok(Box::new(NotIpaddressFunc(IpAddressFunc {
        key,
        values: ip_nets,
    })))
}

fn values_to_ip_nets(name: Name, values: &ValueSet) -> anyhow::Result<Vec<IpNet>> {
    let mut ip_nets = Vec::new();
    for v in &values.0 {
        if let Value::String(s) = v {
            let ip_net: IpNet = s.parse()?;
            ip_nets.push(ip_net);
        } else {
            bail!(
                "value '{}' must be string representation of CIDR for {} condition",
                v,
                name
            );
        }
    }
    ip_nets.sort_unstable();
    Ok(ip_nets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_address_func_evaluate() -> anyhow::Result<()> {
        let func = new_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
        )?;

        let cases = [
            (
                &func,
                HashMap::from([("SourceIp".to_string(), vec!["192.168.1.10".to_string()])]),
                true,
            ),
            (
                &func,
                HashMap::from([("SourceIp".to_string(), vec!["192.168.2.10".to_string()])]),
                false,
            ),
            (&func, HashMap::new(), false),
            (
                &func,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                false,
            ),
        ];

        for (key, values, expected_result) in cases {
            let result = key.evaluate(&values);

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result
            );
        }

        Ok(())
    }

    #[test]
    fn test_ip_address_func_key() -> anyhow::Result<()> {
        let func = new_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
        )?;

        let cases = [(func, AWS_SOURCE_IP)];

        for (key, expected_result) in cases {
            let result = key.key();

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result
            )
        }

        Ok(())
    }

    #[test]
    fn test_ip_address_func_to_map() -> anyhow::Result<()> {
        let func1 = new_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
        )?;
        let func2 = new_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![
                Value::String("192.168.1.0/24".to_string()),
                Value::String("10.1.10.1/32".to_string()),
            ]),
        )?;

        let func1_res = HashMap::from([(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
        )]);
        let func2_res = HashMap::from([(
            AWS_SOURCE_IP,
            ValueSet::new(vec![
                Value::String("192.168.1.0/24".to_string()),
                Value::String("10.1.10.1/32".to_string()),
            ]),
        )]);

        let cases = [(func1, func1_res), (func2, func2_res)];

        for (key, expected_result) in cases {
            let result = key.to_map();

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {:?}, got: {:?}",
                key, expected_result, result
            );
        }

        Ok(())
    }

    fn test_not_ip_address_func_evaluate() -> anyhow::Result<()> {
        let func = new_not_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
        )?;

        let cases = [
            (
                &func,
                HashMap::from([("SourceIp".to_string(), vec!["192.168.2.10".to_string()])]),
                true,
            ),
            (&func, HashMap::new(), true),
            (
                &func,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                true,
            ),
            (
                &func,
                HashMap::from([("SourceIp".to_string(), vec!["192.168.1.10".to_string()])]),
                true,
            ),
        ];

        for (key, values, expected_result) in cases {
            let result = key.evaluate(&values);
            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result
            );
        }

        Ok(())
    }

    #[test]
    fn test_not_ip_address_func_key() -> anyhow::Result<()> {
        let func = new_not_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
        )?;

        let cases = [(func, AWS_SOURCE_IP)];

        for (key, expected_result) in cases {
            let result = key.key();

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result
            );
        }

        Ok(())
    }

    #[test]
    fn test_not_ip_address_func_to_map() -> anyhow::Result<()> {
        let func1 = new_not_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
        )?;
        let func2 = new_not_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![
                Value::String("192.168.1.0/24".to_string()),
                Value::String("10.1.10.1/32".to_string()),
            ]),
        )?;

        let func1_res = HashMap::from([(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
        )]);
        let func2_res = HashMap::from([(
            AWS_SOURCE_IP,
            ValueSet::new(vec![
                Value::String("192.168.1.0/24".to_string()),
                Value::String("10.1.10.1/32".to_string()),
            ]),
        )]);

        let cases = [(func1, func1_res), (func2, func2_res)];

        for (key, expected_result) in cases {
            let result = key.to_map();

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {:?}, got: {:?}",
                key, expected_result, result
            );
        }

        Ok(())
    }

    #[test]
    fn test_new_ip_address_func() -> anyhow::Result<()> {
        let func1 = new_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
        )?;
        let func2 = new_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![
                Value::String("192.168.1.0/24".to_string()),
                Value::String("10.1.10.1/32".to_string()),
            ]),
        )?;

        let cases = [
            (
                AWS_SOURCE_IP,
                ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
                Some(&func1),
                false,
            ),
            (
                AWS_SOURCE_IP,
                ValueSet::new(vec![
                    Value::String("192.168.1.0/24".to_string()),
                    Value::String("10.1.10.1/32".to_string()),
                ]),
                Some(&func2),
                false,
            ),
            // Unsupported key error.
            (
                S3_PREFIX,
                ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
                None,
                true,
            ),
            // Invalid value error.
            (
                AWS_SOURCE_IP,
                ValueSet::new(vec![Value::String("node1.example.org".to_string())]),
                None,
                true,
            ),
            // Invalid CIDR format error.
            (
                AWS_SOURCE_IP,
                ValueSet::new(vec![Value::String("192.168.1.0.0/24".to_string())]),
                None,
                true,
            ),
        ];

        for (key, values, expected_result, expect_err) in cases {
            let key_cache = key.clone();
            let values_cache = values.clone();
            let result = new_ip_address_func(key, values);

            if let Some(expected_result) = expected_result {
                match result {
                    Ok(result) => {
                        assert_eq!(
                            result.to_string(),
                            expected_result.to_string(),
                            "key: '{}', values: '{:?}', expected: {}, got: {}",
                            key_cache,
                            values_cache,
                            expected_result,
                            result
                        )
                    }
                    Err(_) => {
                        assert!(expect_err, "expect an error");
                    }
                }
            };
        }

        Ok(())
    }

    #[test]
    fn test_new_not_ip_address_func() -> anyhow::Result<()> {
        let func1 = new_not_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
        )?;
        let func2 = new_not_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![
                Value::String("192.168.1.0/24".to_string()),
                Value::String("10.1.10.1/32".to_string()),
            ]),
        )?;

        let cases = [
            (
                AWS_SOURCE_IP,
                ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
                Some(&func1),
                false,
            ),
            (
                AWS_SOURCE_IP,
                ValueSet::new(vec![
                    Value::String("192.168.1.0/24".to_string()),
                    Value::String("10.1.10.1/32".to_string()),
                ]),
                Some(&func2),
                false,
            ),
            // Unsupported key error.
            (
                S3_PREFIX,
                ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
                None,
                true,
            ),
            // Invalid value error.
            (
                AWS_SOURCE_IP,
                ValueSet::new(vec![Value::String("node1.example.org".to_string())]),
                None,
                true,
            ),
            // Invalid CIDR format error.
            (
                AWS_SOURCE_IP,
                ValueSet::new(vec![Value::String("192.168.1.0.0/24".to_string())]),
                None,
                true,
            ),
        ];

        for (key, values, expected_result, expect_err) in cases {
            let key_cache = key.clone();
            let values_cache = values.clone();
            let result = new_not_ip_address_func(key, values);

            if let Some(expected_result) = expected_result {
                match result {
                    Ok(result) => {
                        assert_eq!(
                            result.to_string(),
                            expected_result.to_string(),
                            "key: '{}', values: '{:?}', expected: {}, got: {}",
                            key_cache,
                            values_cache,
                            expected_result,
                            result
                        )
                    }
                    Err(_) => {
                        assert!(expect_err, "expect an error");
                    }
                }
            };
        }

        Ok(())
    }
}
