use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::config;
use crate::index_config::IndexConfig;
use crate::index::{self, LocalIndex};

pub struct IndexManager {
    conf: config::Search,
    indicies: RwLock<HashMap<String, Arc<LocalIndex>>>,
}

impl IndexManager {
    pub fn new(conf: config::Search) -> crate::Result<Self> {
        fs::create_dir_all(&conf.data_dir)?;
        Ok(Self {
            conf,
            indicies: RwLock::default(),
        })
    }

    pub async fn create_index(&self, name: String, index_conf: &IndexConfig) -> crate::Result<()> {
        let path = self.index_path(&name);
        fs::create_dir_all(&path)?;
        let index = index::create_index_in_dir(&path, index_conf)?;
        let index = Arc::new(LocalIndex::from_index(name.clone(), index, &self.conf)?);
        self.insert_index(name, index)
    }

    pub async fn delete_index(&self, _name: String) -> crate::Result<()> {
        todo!()
    }

    pub async fn index<'s>(&'s self, name: &str) -> crate::Result<Arc<LocalIndex>> {
        let index = self.indicies
            .read()
            .map_err(crate::error::lock_poisoned)?
            .get(name)
            .cloned();
        if let Some(index) = index {
            Ok(index)
        } else {
            let path = self.index_path(name);
            // TODO: map index not exist error
            // TODO: analyzers not stored in meta.json
            let index = index::open_index_in_dir(&path)?;
            let index = Arc::new(LocalIndex::from_index(name.to_string(), index, &self.conf)?);
            
            self.indicies
                .write()
                .map_err(crate::error::lock_poisoned)?
                .insert(name.to_string(), index.clone());
            
            Ok(index)
        }
    }

    fn insert_index(&self, name: String, index: Arc<LocalIndex>) -> crate::Result<()> {
        self.indicies
            .write()
            .map_err(crate::error::lock_poisoned)?
            .insert(name, index);
        Ok(())
    }

    fn index_path<P: AsRef<Path>>(&self, name: P) -> PathBuf {
        self.conf.data_dir.join(name)
    }
}
