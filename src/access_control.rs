use crate::auth::User;


// TODO: bitflags
#[derive(Debug)]
pub enum Permission {
    READ,
    WRITE,
}


pub struct AccessControlSerivce {}


impl AccessControlSerivce {
    pub fn new_test() -> Self {
        Self {}
    }

    pub fn check_index_access(
        &self,
        user: &User,
        index: &str,
        permission: &Permission
    ) -> crate::Result<()> {
        log::debug!(
            "check user({}) access to index({}) with permissions({:?})",
            user.name, index, permission
        );

        Ok(())
    }
}