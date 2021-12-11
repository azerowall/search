use crate::utils::json_file_storage::JsonFileStorage;
use crate::AppState;
use crate::Result;
use actix_web::{dev::ServiceRequest, web, FromRequest, HttpMessage, HttpRequest};
use actix_web_httpauth::extractors::{
    basic::{BasicAuth, Config},
    AuthenticationError,
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::Entry, HashMap},
    future,
    path::PathBuf,
    sync::RwLock,
};

pub type UserId = String;

#[derive(Debug, Serialize)]
pub struct User {
    name: String,
}

impl User {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn id(&self) -> &UserId {
        &self.name
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

#[derive(Deserialize)]
pub struct AddUserReq {
    pub name: String,
    pub password: String,
}

pub struct AuthService {
    storage: JsonFileStorage<HashMap<String, String>>,
    users: RwLock<HashMap<String, String>>,
}

impl AuthService {
    pub fn new(users_file: PathBuf) -> Result<Self> {
        let storage = JsonFileStorage::new(users_file);
        let users = storage.load()?;
        Ok(Self {
            storage,
            users: RwLock::new(users),
        })
    }

    fn validate_credentials(&self, creds: &BasicAuth) -> bool {
        log::debug!("Try to authenticate user with creds {:?}", creds);
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
                self.storage.store(&users)
            }
        }
    }

    pub fn remove_user(&self, name: &str) -> crate::Result<()> {
        log::info!("Remove user '{}'", name);
        let mut users = self.users.write().map_err(crate::error::lock_poisoned)?;
        users.remove(name);
        self.storage.store(&users)
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
}

pub async fn authentication_handler(
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
