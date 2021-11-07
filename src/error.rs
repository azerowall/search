use actix_rt::blocking::BlockingError;
use actix_web::{
    HttpResponse,
    ResponseError,
    http::StatusCode};
use serde_json::json;
use tantivy::{
    TantivyError,
    schema::DocParsingError,
    query::QueryParserError,
};
use thiserror::Error;


#[derive(Error, Debug)]
pub enum Error {
    #[error("Config: {0}")]
    Config(#[from] config::ConfigError),
    // #[error("Error: {0}")]
    // Actix(#[from] actix_web::Error),
    #[error("IO: {0}")]
    IO(#[from] std::io::Error),

    #[error("Tantivy error: {0}")]
    Tantivy(#[from] TantivyError),
    #[error("Doc parsing error: {0}")]
    DocParsingError(#[from] DocParsingError),
    #[error("Query parsing error: {0}")]
    QueryParsingError(#[from] QueryParserError),
    #[error("Value parsing error: {0}")]
    ValueParsingError(anyhow::Error),
    #[error("Index '{0}' not exist")]
    IndexNotExist(String),
    #[error("Field '{0}' not exist")]
    FieldNotExist(String),
    #[error("Base64 decode error {0}")]
    Base64DecodeError(base64::DecodeError),

    #[error("Lock poisoned")]
    LockPoisoned,

    #[error("Thread pool is gone")]
    Canceled,
}

impl Error {
    pub fn value_parsing_err<E: Into<anyhow::Error>>(err: E) -> Self {
        Self::ValueParsingError(err.into())
    }
}

impl<Guard> From<std::sync::PoisonError<Guard>> for Error {
    fn from(err: std::sync::PoisonError<Guard>) -> Self {
        Self::LockPoisoned
    }
}

impl From<BlockingError<Error>> for Error {
    fn from(err: BlockingError<Error>) -> Self {
        match err {
            BlockingError::Error(e) => e,
            BlockingError::Canceled => Error::Canceled
        }
    }
}


impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        HttpResponse::build(status_code).json(json!({
            "error": {
                "message": self.to_string(),
            }
        }))
    }
}

//pub type Error = anyhow::Error;