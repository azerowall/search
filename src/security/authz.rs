use crate::security::authc::{User, UserId};
use crate::Result;
use anyhow::anyhow;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::Entry, HashMap};
use std::sync::{Arc, RwLock};

bitflags! {
    #[derive(Default, Serialize, Deserialize)]
    pub struct SystemPrivileges: u8 {
        const NONE              = 0b00;
        const MANAGE_SECURITY   = 0b01;
        const MANAGE_INDICES    = 0b10;
    }
    #[derive(Default, Serialize, Deserialize)]
    pub struct IndexPrivileges: u8 {
        const NONE  = 0b00;
        const READ  = 0b01;
        const WRITE = 0b10;
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Permissions {
    system: SystemPrivileges,
    index: HashMap<String, IndexPrivileges>,
}

impl Permissions {
    fn merge(&mut self, other: Permissions) {
        self.system |= other.system;
        for (key, value) in other.index {
            match self.index.entry(key) {
                Entry::Vacant(vacant) => {
                    vacant.insert(value);
                }
                Entry::Occupied(mut occupied) => {
                    *occupied.get_mut() |= value;
                }
            }
        }
    }
}

pub struct Role {
    name: String,
    permissions: Permissions,
}

#[derive(Default)]
struct UserPermissions {
    all: Permissions,
    roles: Vec<Arc<Role>>,
}

impl UserPermissions {
    fn with_role(role: Arc<Role>) -> Self {
        let mut s = Self::default();
        s.add_role(role);
        s
    }
    fn add_role(&mut self, role: Arc<Role>) {
        self.all.merge(role.permissions.clone());
        self.roles.push(role);
    }
    fn remove_role(&mut self, role: &Arc<Role>) {
        self.roles.retain(|r| Arc::ptr_eq(r, role));
    }
}

// TODO: Serialize/Deserialize to/from file
#[derive(Default)]
pub struct PermissionsStorage {
    roles: Vec<Arc<Role>>,
    user_permissions: HashMap<String, UserPermissions>,
}

impl PermissionsStorage {
    fn get_permissions(&self, user: &User) -> Option<&Permissions> {
        self.user_permissions.get(user.id()).map(|perms| &perms.all)
    }

    fn add_role(&mut self, role: Role) {
        self.roles.push(Arc::new(role))
    }

    fn remove_role(&mut self, role: &str) {
        let index = self.roles.iter().position(|r| r.name == role);
        if let Some(index) = index {
            self.roles.remove(index);
        }
    }

    fn get_role(&self, role: &str) -> Option<&Arc<Role>> {
        self.roles
            .iter()
            .position(|r| r.name == role)
            .map(|index| &self.roles[index])
    }

    fn add_role_to_user(&mut self, user: UserId, role: &str) {
        if let Some(role) = self.get_role(role).cloned() {
            match self.user_permissions.entry(user) {
                Entry::Occupied(mut occupied) => {
                    occupied.get_mut().add_role(role);
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(UserPermissions::with_role(role));
                }
            }
        }
    }

    fn remove_role_from_user(&mut self, user: &UserId, role: &str) {
        let Self {
            roles,
            user_permissions,
        } = self;
        let role = roles
            .iter()
            .position(|r| r.name == role)
            .map(|index| &roles[index]);
        if let Some(role) = role {
            if let Some(perms) = user_permissions.get_mut(user) {
                perms.remove_role(role);
            }
        }
    }
}

pub struct AccessControlService {
    storage: RwLock<PermissionsStorage>,
}

impl AccessControlService {
    pub fn new_test() -> Self {
        Self {
            storage: Default::default(),
        }
    }

    pub fn check_system(&self, user: &User, privileges: SystemPrivileges) -> Result<()> {
        let permissions = self.storage.read().map_err(crate::error::lock_poisoned)?;

        let has_permissions = permissions
            .get_permissions(user)
            .map(|perms| perms.system.contains(privileges))
            .unwrap_or(false);

        if has_permissions {
            Ok(())
        } else {
            Err(anyhow!("System privileges [{:?}] required", privileges).into())
        }
    }

    pub fn check_index(
        &self,
        user: &User,
        index: &str,
        privileges: IndexPrivileges,
    ) -> crate::Result<()> {
        let permissions = self.storage.read().map_err(crate::error::lock_poisoned)?;

        let has_permissions = permissions
            .get_permissions(user)
            .map(|perms| perms.index.get(index).copied())
            .flatten()
            .map(|privs| privs.contains(privileges))
            .unwrap_or(false);

        if has_permissions {
            Ok(())
        } else {
            Err(anyhow!("Index privileges [{:?}] required", privileges).into())
        }
    }

    pub fn add_role(&self, role: Role) -> Result<()> {
        let mut permissions = self.storage.write().map_err(crate::error::lock_poisoned)?;
        permissions.add_role(role);
        Ok(())
    }

    pub fn remove_role(&self, role: &str) -> Result<()> {
        let mut permissions = self.storage.write().map_err(crate::error::lock_poisoned)?;
        permissions.remove_role(role);
        Ok(())
    }

    pub fn add_role_to_user(&self, user: &User, role: &str) -> Result<()> {
        let mut permissions = self.storage.write().map_err(crate::error::lock_poisoned)?;
        permissions.add_role_to_user(user.id().clone(), role);
        Ok(())
    }

    pub fn remove_role_from_user(&self, user: &User, role: &str) -> Result<()> {
        let mut permissions = self.storage.write().map_err(crate::error::lock_poisoned)?;
        permissions.remove_role_from_user(user.id(), role);
        Ok(())
    }
}
