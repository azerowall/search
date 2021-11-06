

use std::collections::HashSet;

use actix_web::{Result, error::ErrorForbidden};
use crate::auth::User;


// TODO: bitflags
#[derive(Debug)]
pub enum Permission {
    READ,
    WRITE,
}
pub type PermissionSet = HashSet<Permission>;


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
    ) -> Result<()> {
        log::debug!(
            "check user({}) access to index({}) with permissions({:?})",
            user.name, index, permission
        );

        //Err(ErrorForbidden("user has no access to index"))

        Ok(())
    }
}