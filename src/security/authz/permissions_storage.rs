use super::{IndexPrivileges, Permissions, SystemPrivileges};
use crate::security::authc::{User, UserId};
use crate::Result;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[derive(Default, Clone, Serialize, Deserialize)]
struct UserPermissions(Permissions);

impl UserPermissions {
    pub fn get(&self) -> Permissions {
        self.0.clone()
    }
}

impl From<Permissions> for UserPermissions {
    fn from(perms: Permissions) -> Self {
        Self(perms)
    }
}

#[derive(Default, Serialize, Deserialize)]
struct DACModel {
    user_permissions: HashMap<UserId, UserPermissions>,
}

impl DACModel {
    fn load(path: &Path) -> Result<Self> {
        match File::open(path) {
            Ok(file) => serde_json::from_reader(file).map_err(From::from),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Default::default()),
            Err(err) => Err(err.into()),
        }
    }
    fn store(&self, path: &Path) -> Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserPermissionsInfo {
    user: UserId,
    permissions: UserPermissions,
}

pub struct PermissionsStorage {
    path: PathBuf,
    model: RwLock<DACModel>,
}

impl PermissionsStorage {
    pub fn new(path: PathBuf) -> Result<Self> {
        let model = DACModel::load(&path)?;
        Ok(Self {
            path,
            model: RwLock::new(model),
        })
    }

    pub fn check_system(&self, user: &User, privileges: SystemPrivileges) -> Result<()> {
        let has_permissions = self
            .get_permissions(user)
            .map(|perms| perms.check_system(privileges))
            .unwrap_or(false);

        if has_permissions {
            Ok(())
        } else {
            Err(anyhow!("System privileges [{}] required", privileges).into())
        }
    }

    pub fn check_index(&self, user: &User, index: &str, privileges: IndexPrivileges) -> Result<()> {
        let has_permissions = self
            .get_permissions(user)
            .map(|perms| perms.check_index(index, privileges))
            .unwrap_or(false);

        if has_permissions {
            Ok(())
        } else {
            Err(anyhow!("Index privileges [{}] required", privileges).into())
        }
    }

    pub fn get_permissions(&self, user: &User) -> Option<Permissions> {
        let model = self.model.read().unwrap();
        model
            .user_permissions
            .get(user.id())
            .map(|perms| perms.get())
    }

    pub fn assign_permissions(&self, user: UserId, permissions: Permissions) -> Result<()> {
        log::info!("Assign permissions to user {}", &user);
        let mut model = self.model.write().unwrap();
        match model.user_permissions.entry(user) {
            Entry::Vacant(vacant) => {
                vacant.insert(permissions.into());
            }
            Entry::Occupied(mut occupied) => {
                *occupied.get_mut() = permissions.into();
            }
        }
        model.store(&self.path)
    }

    pub fn list_users_permissions(&self) -> Vec<UserPermissionsInfo> {
        let model = self.model.read().unwrap();
        model
            .user_permissions
            .iter()
            .map(|(user, perms)| UserPermissionsInfo {
                user: user.clone(),
                permissions: perms.clone(),
            })
            .collect()
    }

    pub fn remove_user(&self, user: &UserId) {
        let mut model = self.model.write().unwrap();
        model.user_permissions.remove(user);
    }
}
