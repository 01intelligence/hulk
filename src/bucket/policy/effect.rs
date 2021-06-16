// Policy statement effect Allow or Deny.
pub type Effect<'a> = &'a str;

pub const ALLOW: Effect = "Allow"; // allow effect
pub const DENY: Effect = "Deny"; // deny effect

impl<'a> super::Allowed for Effect<'a> {
    fn is_allowed(&self, b: bool) -> bool {
        if *self == ALLOW {
            b
        } else {
            !b
        }
    }
}

impl<'a> super::Valid for Effect<'a> {
    fn is_valid(&self) -> bool {
        match *self {
            ALLOW | DENY => true,
            _ => false,
        }
    }
}
