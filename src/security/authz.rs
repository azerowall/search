use crate::security::authc::{User, UserId};
use crate::Result;
use anyhow::anyhow;
use bitflags::bitflags;
use serde::de::Visitor;
use serde::ser::{SerializeMap, SerializeStruct};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{hash_map::Entry, HashMap};
use std::fs::File;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
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
    pub fn all() -> Self {
        Self {
            system: SystemPrivileges::all(),
            index: Default::default(), // TODO
        }
    }
    pub fn merge(&mut self, other: Permissions) {
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

    pub fn check_system(&self, privs: SystemPrivileges) -> bool {
        self.system.contains(privs)
    }

    pub fn check_index(&self, index: &str, privs: IndexPrivileges) -> bool {
        self.index
            .get(index)
            .map(|p| p.contains(privs))
            .unwrap_or(false)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Role {
    name: String,
    permissions: Permissions,
}

impl Role {
    pub fn new(name: String, permissions: Permissions) -> Self {
        Self { name, permissions }
    }
}

#[derive(Default)]
pub struct Roles(HashMap<String, Arc<Role>>);

impl Roles {
    fn get_roles<'a, I: Iterator<Item = &'a str>>(&self, iter: I) -> Self {
        Self::from_iter(iter.flat_map(|role| self.0.get(role).cloned()))
    }
}

impl FromIterator<Arc<Role>> for Roles {
    fn from_iter<T: IntoIterator<Item = Arc<Role>>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .map(|role| (role.name.clone(), role))
                .collect(),
        )
    }
}

// impl Roles {
//     fn index(&self, role: &str) -> Option<usize> {
//         self.0.iter().position(|r| r.name == role)
//     }
//     fn get_ref(&self, role: &str) -> Option<&Arc<Role>> {
//         self.index(role).map(|index| &self.0[index])
//     }
//     fn get(&self, role: &str) -> Option<Arc<Role>> {
//         self.get_ref(role).cloned()
//     }
//     fn add(&mut self, role: Arc<Role>) {
//         self.0.push(role);
//     }
//     fn remove(&mut self, role: &str) {
//         if let Some(index) = self.index(role) {
//             self.0.remove(index);
//         }
//     }
// }

#[derive(Default)]
struct UserPermissions {
    roles: Roles,
}

impl UserPermissions {
    fn new(roles: Roles) -> Self {
        Self { roles }
    }
    fn with_role(role: Arc<Role>) -> Self {
        let mut s = Self::default();
        s.add_role(role);
        s
    }
    fn add_role(&mut self, role: Arc<Role>) {
        self.roles.0.insert(role.name.clone(), role);
    }
    fn remove_role(&mut self, role: &str) {
        self.roles.0.remove(role);
    }
    fn get(&self) -> Permissions {
        let mut perms = Permissions::default();
        for (_, role) in &self.roles.0 {
            perms.merge(role.permissions.clone());
        }
        perms
    }
}

impl Extend<Arc<Role>> for UserPermissions {
    fn extend<I: IntoIterator<Item = Arc<Role>>>(&mut self, iter: I) {
        for role in iter {
            self.add_role(role);
        }
    }
}

impl FromIterator<Arc<Role>> for UserPermissions {
    fn from_iter<I: IntoIterator<Item = Arc<Role>>>(iter: I) -> Self {
        let mut s = Self::default();
        s.extend(iter);
        s
    }
}

#[derive(Default)]
pub struct RBACModel {
    roles: Roles,
    user_permissions: HashMap<UserId, UserPermissions>,
}

impl RBACModel {
    fn load(path: &Path) -> Self {
        let file = File::open("./permissions.json").unwrap();
        serde_json::from_reader(file).unwrap()
    }
    fn store(&self, path: &Path) {
        let file = File::create("./permissions.json").unwrap();
        serde_json::to_writer(file, self).unwrap();
    }
}

impl Serialize for RBACModel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("PermissionsStorage", 2)?;
        // TODO: remove unnecessary collect()
        state.serialize_field(
            "roles",
            &self
                .roles
                .0
                .iter()
                .map(|(key, role)| role.as_ref())
                .collect::<Vec<_>>(),
        )?;
        state.serialize_field(
            "user_roles",
            &self
                .user_permissions
                .iter()
                .map(|(user, perms)| {
                    let perms = perms
                        .roles
                        .0
                        .iter()
                        .map(|(key, role)| &role.name)
                        .collect::<Vec<_>>();
                    (user, perms)
                })
                .collect::<HashMap<_, _>>(),
        )?;
        state.end()
    }
}

struct RBACModelVisitor;

impl<'de> Visitor<'de> for RBACModelVisitor {
    type Value = RBACModel;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Roles and user-role mapping")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum Field {
            Roles,
            UserRoles,
        }
        let mut roles = Roles::default();
        let mut user_roles_mapping: HashMap<&str, Vec<&str>> = Default::default();
        while let Some(key) = map.next_key::<Field>()? {
            match key {
                Field::Roles => {
                    let roles_ir: Vec<Role> = map.next_value()?;
                    roles.0 = roles_ir
                        .into_iter()
                        .map(|r| (r.name.clone(), Arc::new(r)))
                        .collect::<HashMap<_, _>>();
                }
                Field::UserRoles => {
                    user_roles_mapping = map.next_value()?;
                }
            }
        }

        let user_permissions: HashMap<UserId, UserPermissions> = user_roles_mapping
            .into_iter()
            .map(|(user, user_roles)| {
                let roles = roles.get_roles(user_roles.into_iter());
                (user.into(), UserPermissions::new(roles))
            })
            .collect();

        Ok(RBACModel {
            roles,
            user_permissions,
        })
    }
}

impl<'de> Deserialize<'de> for RBACModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            "PermissionsStorage",
            &["roles", "user_roles"],
            RBACModelVisitor,
        )
    }
}

pub struct PermissionStorage {
    path: PathBuf,
    model: RwLock<RBACModel>,
}

impl PermissionStorage {
    fn new(path: PathBuf) -> Self {
        let model = RBACModel::load(&path);
        Self {
            path,
            model: RwLock::new(model),
        }
    }

    fn get_permissions(&self, user: &User) -> Option<Permissions> {
        let model = self.model.read().unwrap();
        model
            .user_permissions
            .get(user.id())
            .map(|perms| perms.get())
    }

    fn add_role(&self, role: Role) {
        let mut model = self.model.write().unwrap();
        model.roles.0.insert(role.name.clone(), Arc::new(role));
        model.store(&self.path);
    }

    fn remove_role(&self, role: &str) {
        let mut model = self.model.write().unwrap();
        model.roles.0.remove(role);
        todo!()
    }

    fn add_role_to_user(&self, user: UserId, role: &str) {
        let mut model = self.model.write().unwrap();
        if let Some(role) = model.roles.0.get(role).cloned() {
            match model.user_permissions.entry(user) {
                Entry::Occupied(mut occupied) => {
                    occupied.get_mut().add_role(role);
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(UserPermissions::with_role(role));
                }
            }
        }
        model.store(&self.path);
    }

    fn remove_role_from_user(&self, user: &UserId, role: &str) {
        let mut model = self.model.write().unwrap();
        if let Some(perms) = model.user_permissions.get_mut(user) {
            perms.remove_role(role);
        }
        model.store(&self.path);
    }
}

pub struct AccessControlService {
    storage: PermissionStorage,
}

impl AccessControlService {
    pub fn new_test() -> Self {
        Self {
            storage: PermissionStorage::new("./permissions.json".into()),
        }
    }

    pub fn check_system(&self, user: &User, privileges: SystemPrivileges) -> Result<()> {
        let has_permissions = self
            .storage
            .get_permissions(user)
            .map(|perms| perms.check_system(privileges))
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
        let has_permissions = self
            .storage
            .get_permissions(user)
            .map(|perms| perms.check_index(index, privileges))
            .unwrap_or(false);

        if has_permissions {
            Ok(())
        } else {
            Err(anyhow!("Index privileges [{:?}] required", privileges).into())
        }
    }

    pub fn add_role(&self, role: Role) -> Result<()> {
        self.storage.add_role(role);
        Ok(())
    }

    pub fn remove_role(&self, role: &str) -> Result<()> {
        self.storage.remove_role(role);
        Ok(())
    }

    pub fn add_role_to_user(&self, user: &User, role: &str) -> Result<()> {
        self.storage.add_role_to_user(user.id().clone(), role);
        Ok(())
    }

    pub fn remove_role_from_user(&self, user: &User, role: &str) -> Result<()> {
        self.storage.remove_role_from_user(user.id(), role);
        Ok(())
    }
}
