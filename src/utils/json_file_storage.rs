use std::{fs::File, marker::PhantomData, path::PathBuf};

use serde::{de::DeserializeOwned, Serialize};

use crate::Result;

pub struct JsonFileStorage<T> {
    path: PathBuf,
    _phantom: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned + Default> JsonFileStorage<T> {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            _phantom: PhantomData,
        }
    }
    pub fn load(&self) -> Result<T> {
        match File::open(&self.path) {
            Ok(file) => serde_json::from_reader(file).map_err(From::from),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Default::default()),
            Err(err) => Err(err.into()),
        }
    }
    pub fn store(&self, value: &T) -> Result<()> {
        let file = File::create(&self.path)?;
        serde_json::to_writer(file, value)?;
        Ok(())
    }
}
