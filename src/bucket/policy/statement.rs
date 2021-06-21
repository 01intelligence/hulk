use super::*;

pub struct Statement<'a> {
    pub sid: ID,
    pub effect: Effect<'a>,
}