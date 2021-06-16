// Policy ID.
pub type ID = String;

impl super::Valid for ID {
    fn is_valid(&self) -> bool {
        std::str::from_utf8(self.as_bytes()).is_ok()
    }
}
