use std::fmt;

use anyhow::anyhow;

use actix_web::{
    HttpResponse,
    ResponseError,
    http::StatusCode
};
use serde_json::json;


#[derive(Debug)]
pub struct Error {
    status_code: StatusCode,
    err: anyhow::Error,
}

impl Error {
    fn internal(err: anyhow::Error) -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            err,
        }
    }
    fn not_found(err: anyhow::Error) -> Self {
        Self {
            status_code: StatusCode::NOT_FOUND,
            err
        }
    }
    fn bad_request(err: anyhow::Error) -> Self {
        Self {
            status_code: StatusCode::BAD_REQUEST,
            err
        }
    }
}

pub fn lock_poisoned<Guard>(_err: std::sync::PoisonError<Guard>) -> Error {
    Error::internal(anyhow!("Lock poisoned"))
}
pub fn index_not_exist(index: String) -> Error {
    Error::not_found(anyhow!("Index '{0}' not exist", index))
}
pub fn field_not_exist(field: String) -> Error {
    Error::bad_request(anyhow!("Field '{0}' not exist", field))
}
pub fn value_parsing_err<E: Into<anyhow::Error>>(err: E) -> Error {
    Error::bad_request(err.into())
}
pub fn invalid_index_name(name: String) -> Error {
    Error::bad_request(anyhow!(name))
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.err.fmt(f)
    }
}

impl<E: Into<anyhow::Error> + Send> From<E> for Error {
    fn from(err: E) -> Self {
        Self::internal(err.into())
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        self.status_code
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
