use super::*;

pub struct Statement<'a, 'b> {
    pub sid: ID,
    pub effect: Effect<'a>,
    pub principal: Principal,
    pub actions: ActionSet<'b>,
    pub resources: ResourceSet,
    pub conditions: condition::Functions,
}
