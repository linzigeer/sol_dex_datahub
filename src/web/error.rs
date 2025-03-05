use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

pub enum WebAppError {
    UnAuthorized { err_msg: String },
    InvalidSignature,
    InvalidRequest { err_msg: String },
    Other { err_msg: String },
}

#[derive(Debug, Serialize)]
pub struct ErrorResp {
    error: String,
}

impl WebAppError {
    pub fn invalid_req(err_msg: impl Into<String>) -> Self {
        let err_msg = err_msg.into();
        WebAppError::InvalidRequest { err_msg }
    }

    pub fn unauth(err_msg: impl Into<String>) -> Self {
        WebAppError::UnAuthorized {
            err_msg: err_msg.into(),
        }
    }

    pub fn other(err_msg: impl Into<String>) -> Self {
        let err_msg = err_msg.into();
        WebAppError::Other { err_msg }
    }
}

impl IntoResponse for WebAppError {
    fn into_response(self) -> Response {
        match self {
            Self::UnAuthorized { err_msg } => {
                // let err_msg = "UnAuthorized".to_string();
                let mut resp = Json(ErrorResp { error: err_msg }).into_response();
                *resp.status_mut() = StatusCode::UNAUTHORIZED;
                resp
            }
            Self::InvalidSignature => {
                let err_msg = "Invalid signature".to_string();
                let mut resp = Json(ErrorResp { error: err_msg }).into_response();
                *resp.status_mut() = StatusCode::BAD_REQUEST;
                resp
            }
            Self::InvalidRequest { err_msg } => {
                let mut resp = Json(ErrorResp { error: err_msg }).into_response();
                *resp.status_mut() = StatusCode::BAD_REQUEST;
                resp
            }
            Self::Other { err_msg } => {
                let mut resp = Json(ErrorResp { error: err_msg }).into_response();
                *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                resp
            }
        }
    }
}

impl<E> From<E> for WebAppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        let err_msg = format!("{}", err.into());
        Self::Other { err_msg }
    }
}
