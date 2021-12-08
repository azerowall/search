use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::config;
use crate::index::LocalIndex;
use crate::index_config::IndexConfig;

pub struct IndexManager {
    conf: config::Search,
    indices: RwLock<HashMap<String, Arc<LocalIndex>>>,
}

impl IndexManager {
    pub fn new(conf: config::Search) -> crate::Result<Self> {
        fs::create_dir_all(&conf.data_dir)?;
        Ok(Self {
            conf,
            indices: RwLock::default(),
        })
    }

    pub async fn create_index(&self, name: String, index_conf: &IndexConfig) -> crate::Result<()> {
        let path = self.index_path(&name)?;
        fs::create_dir_all(&path)?;
        let index = LocalIndex::creare_in_dir(&path, index_conf, &self.conf)?;
        let index = Arc::new(index);
        self.insert_index(name, index)
    }

    pub async fn delete_index(&self, name: &str) -> crate::Result<()> {
        self.indices
            .write()
            .map_err(crate::error::lock_poisoned)?
            .remove(name);
        let path = self.index_path(name)?;
        fs::remove_dir_all(path)?;
        Ok(())
    }

    pub async fn index<'s>(&'s self, name: &str) -> crate::Result<Arc<LocalIndex>> {
        let index = self
            .indices
            .read()
            .map_err(crate::error::lock_poisoned)?
            .get(name)
            .cloned();
        if let Some(index) = index {
            Ok(index)
        } else {
            let path = self.index_path(name)?;
            // TODO: map index not exist error
            let index = LocalIndex::open_in_dir(&path, &self.conf)?;
            let index = Arc::new(index);

            self.insert_index(name.to_string(), index.clone())?;

            Ok(index)
        }
    }

    fn insert_index(&self, name: String, index: Arc<LocalIndex>) -> crate::Result<()> {
        self.indices
            .write()
            .map_err(crate::error::lock_poisoned)?
            .insert(name, index);
        Ok(())
    }

    fn index_path(&self, name: &str) -> crate::Result<PathBuf> {
        if !name
            .chars()
            .all(|c: char| c.is_ascii_alphanumeric() || c == '_')
        {
            Err(crate::error::invalid_index_name(name.to_string()))
        } else {
            Ok(self.conf.data_dir.join(name))
        }
    }
}
