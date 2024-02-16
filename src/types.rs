use anyhow::Error;
use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::Value;
use sqlx::SqlitePool;
use tracing::{debug, error};

#[derive(Clone)]
pub struct AEState {
    pub db_pool: SqlitePool,
    pub settings: Value,
}

#[derive(Debug)]
pub struct AeError(Error);
impl IntoResponse for AeError {
    fn into_response(self) -> Response {
        let error_str = self.0.to_string();
        error!("AE error: {:#?}", error_str);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Res::new().status(1).msg(self.0.to_string()),
        )
            .into_response()
    }
}
impl<E> From<E> for AeError
where
    E: Into<Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Res {
    pub status: usize,
    pub msg: String,
    pub data: Option<Value>,
}
pub fn ok(data: Value) -> Result<Res, AeError> {
    Ok(Res::new().msg("success".to_string()).data(Some(data)))
}
pub fn err(reason: String) -> Result<Res, AeError> {
    Ok(Res::new().status(1).msg(reason))
}
impl Res {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn status(mut self, status: usize) -> Self {
        self.status = status;
        self
    }
    pub fn msg(mut self, msg: String) -> Self {
        self.msg = msg;
        self
    }
    pub fn data(mut self, data: Option<Value>) -> Self {
        self.data = data;
        self
    }
}
impl IntoResponse for Res {
    fn into_response(self) -> Response {
        debug!("payload: {:#?}", &self);
        Json(self).into_response()
    }
}
impl Default for Res {
    fn default() -> Self {
        Self {
            status: 0,
            msg: String::new(),
            data: None,
        }
    }
}
