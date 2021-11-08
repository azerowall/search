use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::config;
use crate::index::{IndexConfig, LocalIndex};

pub struct IndexManager {
    data_dir: PathBuf,
    indicies: RwLock<HashMap<String, Arc<LocalIndex>>>,
}

impl IndexManager {
    pub fn load_from(data_dir: PathBuf) -> crate::Result<Self> {
        fs::create_dir_all(&data_dir)?;
        Ok(Self {
            data_dir,
            indicies: RwLock::default(),
        })
    }

    pub async fn create_index(&self, name: String, index_conf: IndexConfig, conf: &config::Search) -> crate::Result<()> {
        let path = self.index_path(&name);
        fs::create_dir_all(&path)?;

        let IndexConfig { settings, schema } = index_conf;
        let index = tantivy::Index::builder()
            .settings(settings)
            .schema(schema)
            .create_in_dir(&path)?;
        let index = Arc::new(LocalIndex::from_index(name.clone(), index, conf)?);
        self.insert_index(name, index)
    }

    pub async fn delete_index(&self, _name: String) -> crate::Result<()> {
        todo!()
    }

    pub async fn index<'s>(&'s self, name: &str) -> crate::Result<Arc<LocalIndex>> {
        let index = self
            .indicies
            .read()
            .map_err(crate::error::lock_poisoned)?
            .get(name)
            .ok_or_else(|| crate::error::index_not_exist(name.to_owned()))?
            .clone();
        Ok(index)
    }

    fn insert_index(&self, name: String, index: Arc<LocalIndex>) -> crate::Result<()> {
        self.indicies
            .write()
            .map_err(crate::error::lock_poisoned)?
            .insert(name, index);
        Ok(())
    }

    fn index_path<P: AsRef<Path>>(&self, name: P) -> PathBuf {
        self.data_dir.join(name)
    }
}
