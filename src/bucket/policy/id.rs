// Policy ID.
pub type ID = String;

impl super::Valid for ID {
    fn is_valid(&self) -> bool {
        std::str::from_utf8(self.as_bytes()).is_ok()
    }
}

mod tests {
    use super::super::Valid;
    use super::*;

    #[test]
    fn test_id_is_valid() {
        let cases = [
            (ID::from("DenyEncryptionSt1"), true),
            (ID::from(""), true),
            (
                unsafe { ID::from_utf8_unchecked(vec![b'a', b'a', b'\xe2']) },
                false,
            ),
        ];

        for (key, expected_result) in cases.iter() {
            assert_eq!(
                key.is_valid(),
                *expected_result,
                "key: '{:?}', expected: {}, got: {}",
                key,
                expected_result,
                key.is_valid()
            );
        }
    }
}
