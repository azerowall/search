use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use anyhow::{anyhow, Context};

use crate::index::{IndexConfig, LocalIndex};

pub struct IndexManager {
    root: PathBuf,
    indicies: RwLock<HashMap<String, Arc<LocalIndex>>>,
}

impl IndexManager {
    pub fn load_from(root: PathBuf) -> crate::Result<Self> {
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            indicies: RwLock::default(),
        })
    }

    pub async fn create_index(&self, name: String, conf: IndexConfig) -> crate::Result<()> {
        let path = self.index_path(&name);
        fs::create_dir_all(&path)?;

        let index = configured_builder(conf).create_in_dir(&path)?;
        let index = Arc::new(LocalIndex::from_index(name.clone(), index)?);
        self.insert_index(name, index)
    }

    pub async fn delete_index(&self, _name: String) -> crate::Result<()> {
        todo!()
    }

    pub async fn index<'s>(&'s self, name: &str) -> crate::Result<Arc<LocalIndex>> {
        let index = self
            .indicies
            .read()
            .map_err(|_| anyhow!("poison error"))?
            .get(name)
            .context(format!("index '{}' not exist", name))?
            .clone();
        Ok(index)
    }

    fn insert_index(&self, name: String, index: Arc<LocalIndex>) -> crate::Result<()> {
        self.indicies
            .write()
            .map_err(|_| anyhow!("poison error"))?
            .insert(name, index);
        Ok(())
    }

    fn index_path<P: AsRef<Path>>(&self, name: P) -> PathBuf {
        self.root.join(name)
    }

    #[cfg(test)]
    pub fn create_test_index_in_ram(
        &self,
        name: String,
        schema: tantivy::schema::Schema,
    ) -> crate::Result<()> {
        let conf = IndexConfig {
            settings: Default::default(),
            schema: schema,
        };
        let index = configured_builder(conf).create_in_ram()?;
        let index = Arc::new(LocalIndex::from_index(name.clone(), index)?);
        self.insert_index(name, index)
    }
}

fn configured_builder(conf: IndexConfig) -> tantivy::IndexBuilder {
    let IndexConfig { settings, schema } = conf;

    tantivy::Index::builder().settings(settings).schema(schema)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test;

    #[actix_rt::test]
    async fn test_create() -> crate::Result<()> {
        let indicies = IndexManager::load_from("/tmp/test".into())?;
        let index_name = String::from("test");
        let schema = test::make_test_schema();

        let conf = IndexConfig {
            settings: Default::default(),
            schema,
        };

        assert!(indicies.index(&index_name).await.is_err());
        indicies.create_index(index_name.clone(), conf).await?;
        assert!(indicies.index(&index_name).await.is_ok());

        Ok(())
    }
}
