use crate::impl_flags_serde;
use crate::utils::flags::AllFlags;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::Entry, HashMap};

// TODO: consider using enumflags2
bitflags! {
    #[derive(Default)]
    pub struct SystemPrivileges: u8 {
        const NONE              = 0b00;
        const MANAGE_SECURITY   = 0b01;
        const MANAGE_INDICES    = 0b10;
    }
    #[derive(Default)]
    pub struct IndexPrivileges: u8 {
        const NONE  = 0b00;
        const READ  = 0b01;
        const WRITE = 0b10;
    }
}

impl AllFlags for SystemPrivileges {
    fn all_flags() -> &'static [(Self, &'static str)] {
        &[
            (Self::MANAGE_SECURITY, "manage_security"),
            (Self::MANAGE_INDICES, "manage_indices"),
        ]
    }
}

impl AllFlags for IndexPrivileges {
    fn all_flags() -> &'static [(Self, &'static str)] {
        &[(Self::READ, "read"), (Self::WRITE, "write")]
    }
}

impl_flags_serde!(SystemPrivileges);
impl_flags_serde!(IndexPrivileges);

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

    pub fn none() -> Self {
        Self::default()
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
