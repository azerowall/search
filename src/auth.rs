use std::{
    collections::{hash_map::Entry, HashMap},
    fs::{self, File},
    future,
    path::PathBuf,
    sync::RwLock,
};

use actix_web::{dev::ServiceRequest, web, FromRequest, HttpMessage, HttpRequest};

use actix_web_httpauth::extractors::{
    basic::{BasicAuth, Config},
    AuthenticationError,
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::Result;

#[derive(Debug, Serialize)]
pub struct User {
    pub name: String,
}

#[derive(Deserialize)]
pub struct AddUserReq {
    pub name: String,
    pub password: String,
}

impl User {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl FromRequest for User {
    type Error = actix_web::Error;
    type Future = future::Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let user = req.extensions_mut().remove::<User>().unwrap();

        future::ready(Ok(user))
    }
}

pub struct AuthService {
    users: RwLock<HashMap<String, String>>,
    users_file: PathBuf,
}

impl AuthService {
    pub fn new(users_file: PathBuf) -> Result<Self> {
        let s = Self {
            users: RwLock::new(HashMap::new()),
            users_file,
        };
        s.load_from_file()?;
        Ok(s)
    }

    fn validate_credentials(&self, creds: &BasicAuth) -> bool {
        let users = self.users.read().unwrap();
        let valid_password = users.get(creds.user_id().as_ref());

        match (creds.password(), valid_password) {
            (Some(password), Some(valid_password)) if password == valid_password => true,
            _ => false,
        }
    }

    pub fn add_user(&self, AddUserReq { name, password }: AddUserReq) -> Result<()> {
        log::info!("Add user {}", name);
        let mut users = self.users.write().map_err(crate::error::lock_poisoned)?;
        match users.entry(name) {
            Entry::Occupied(_) => Err(anyhow!("User already exists").into()),
            Entry::Vacant(v) => {
                v.insert(password);
                self.save_to_file(&*users)
            }
        }
    }

    pub fn remove_user(&self, name: &str) -> crate::Result<()> {
        log::info!("Remove user '{}'", name);
        let mut users = self.users.write().map_err(crate::error::lock_poisoned)?;
        users.remove(name);
        self.save_to_file(&*users)
    }

    pub fn list_users(&self) -> Result<Vec<User>> {
        let list = self
            .users
            .read()
            .map_err(crate::error::lock_poisoned)?
            .keys()
            .cloned()
            .map(User::new)
            .collect();
        Ok(list)
    }

    fn load_from_file(&self) -> crate::Result<()> {
        if self.users_file.exists() {
            let s = fs::read_to_string(&self.users_file)?;
            let mut users = self.users.write().map_err(crate::error::lock_poisoned)?;
            *users = serde_json::from_str(&s)?;
        }
        Ok(())
    }

    fn save_to_file(&self, users: &HashMap<String, String>) -> Result<()> {
        let file = File::create(&self.users_file)?;
        serde_json::to_writer(file, &*users)?;
        Ok(())
    }
}

pub async fn validator(
    req: ServiceRequest,
    creds: BasicAuth,
) -> Result<ServiceRequest, actix_web::Error> {
    let state = req.app_data::<web::Data<AppState>>().unwrap();

    if state.auth.validate_credentials(&creds) {
        req.extensions_mut()
            .insert(User::new(creds.user_id().to_string()));
        Ok(req)
    } else {
        let config = req
            .app_data::<Config>()
            .map(|conf| conf.clone())
            .unwrap_or_default();
        Err(AuthenticationError::from(config).into())
    }
}
