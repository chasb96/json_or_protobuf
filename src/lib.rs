use std::{error::Error, fmt::{self, Display}};

use axum::{async_trait, extract::{FromRequest, Request}, http::{header::{ACCEPT, CONTENT_TYPE}, HeaderMap, StatusCode}, response::{IntoResponse, Response}, Json, RequestExt};
use axum_extra::protobuf::Protobuf;
use prost::Message;
use serde::Serialize;

const CONTENT_TYPE_PROTOBUF: &'static str = "application/octet-stream";
const CONTENT_TYPE_JSON: &'static str = "application/json";

pub enum JsonOrProtobuf<T> {
    Protobuf(T),
    Json(T),
}

#[derive(Debug)]
pub struct ContentTypeError(String);

impl Error for ContentTypeError {}

impl Display for ContentTypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid Content-Type {}", self.0)
    }
}

impl<T> JsonOrProtobuf<T> {
    pub fn new(body: T, content_type: &str) -> Result<Self, ContentTypeError> {
        match content_type {
            CONTENT_TYPE_PROTOBUF => Ok(Self::Protobuf(body)),
            CONTENT_TYPE_JSON => Ok(Self::Json(body)),
            _ => Err(ContentTypeError(content_type.to_string()))
        }
    }

    pub fn from_accept_header(body: T, headers: &HeaderMap) -> Self {
        let accept = headers
            .get(ACCEPT)
            .and_then(|header_value| header_value.to_str().ok());

        if accept == Some(CONTENT_TYPE_PROTOBUF) {
            Self::Protobuf(body)
        } else {
            Self::Json(body)
        }
    }

    pub fn decompose(self) -> (T, String) {
        match self {
            JsonOrProtobuf::Protobuf(body) => (body, CONTENT_TYPE_PROTOBUF.to_string()),
            JsonOrProtobuf::Json(body) => (body, CONTENT_TYPE_JSON.to_string()),
        }
    }
}

impl<T> TryFrom<(T, String)> for JsonOrProtobuf<T> {
    type Error = ContentTypeError;
    
    fn try_from(value: (T, String)) -> Result<Self, Self::Error> {
        Self::new(value.0, &value.1)
    }
}

impl<T> Into<(T, String)> for JsonOrProtobuf<T> {
    fn into(self) -> (T, String) {
        self.decompose()
    }
}

#[async_trait]
impl<'a, T, S> FromRequest<S> for JsonOrProtobuf<T> 
where
    T: 'static,
    Json<T>: FromRequest<()>,
    Protobuf<T>: FromRequest<()>,
    S: Send + Sync
{
    type Rejection = StatusCode;

    async fn from_request(request: Request, _: &S) -> Result<Self, Self::Rejection> {
        let content_type = request
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|content_type| content_type.to_str().ok());

        match content_type {
            Some("application/octet-stream") => {
                let Protobuf(payload) = request
                    .extract::<Protobuf<T>,_>()
                    .await
                    .map_err(|_| StatusCode::BAD_REQUEST)?;

                Ok(Self::Protobuf(payload))
            },
            Some("application/json") => {
                let Json(payload) = request
                    .extract::<Json<T>, _>()
                    .await
                    .map_err(|_| StatusCode::BAD_REQUEST)?;

                Ok(Self::Json(payload))
            },
            _ => Err(StatusCode::BAD_REQUEST),
        }
    }
}

impl<T> IntoResponse for JsonOrProtobuf<T> 
where
    T: Serialize + Message + Default
{
    fn into_response(self) -> Response {
        match self {
            JsonOrProtobuf::Protobuf(p) => Protobuf(p).into_response(),
            JsonOrProtobuf::Json(j) => Json(j).into_response(),
        }
    }
}