use std::{collections::HashMap, future};

use actix_web::{Error, FromRequest, HttpMessage, HttpRequest, Result, dev::ServiceRequest, web};

use actix_web_httpauth::{
    extractors::{
        AuthenticationError,
        basic::{BasicAuth, Config},
    },
};

use crate::AppState;

#[derive(Debug)]
pub struct User {
    pub name: String,
}

impl User {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl FromRequest for User {
    type Error = Error;
    type Future = future::Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let user = req
            .extensions_mut()
            .remove::<User>()
            .unwrap();
        
        future::ready(Ok(user))
    }
}

pub struct AuthService {
    users: HashMap<String, String>,
}

impl AuthService {
    pub fn new_test() -> Self {
        let mut users = HashMap::new();
        users.insert("test".into(), "test".into());

        Self { users }
    }

    fn validate_credentials(&self, creds: &BasicAuth) -> bool {
        let valid_password = self.users.get(creds.user_id().as_ref());

        match (creds.password(), valid_password) {
            (Some(password), Some(valid_password))
                if password == valid_password => true,
            _ => false,
        }
    }
}

pub async fn validator(
    req: ServiceRequest,
    creds: BasicAuth,
) -> Result<ServiceRequest> {
    let state = req.app_data::<web::Data<AppState>>().unwrap();
    
    if state.auth.validate_credentials(&creds) {
        req
            .extensions_mut()
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

