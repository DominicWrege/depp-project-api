use crate::api::{IliasId, SubmissionExample};
use actix_web::error::JsonPayloadError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use grpc_api::AssignmentId;
use uuid::Uuid;
#[derive(serde::Serialize)]
pub struct ErrJson {
    msg: String,
}
#[derive(serde::Serialize)]
pub struct ErrSubmission {
    msg: String,
    example: SubmissionExample,
}

impl From<&Error> for ErrJson {
    fn from(e: &Error) -> Self {
        ErrJson { msg: e.to_string() }
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::DuplicateIliasId => StatusCode::CONFLICT,
            Error::NotFoundIliasId(_) | Error::NotAssignment(_) => StatusCode::NOT_FOUND,
            Error::BadRequest => StatusCode::BAD_REQUEST,
            Error::Body(_err) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let err = ErrJson::from(self);
        let code = self.status_code();
        let mut response = HttpResponse::build(code);
        match self {
            Error::DuplicateIliasId | Error::NotFoundIliasId(_) | Error::NotAssignment(_) => {
                response.json(err)
            }
            Error::Body(err) => response.json(ErrSubmission {
                msg: err.to_string(),
                example: SubmissionExample::new(
                    IliasId::default(),
                    "ZWNobyAiSGFsbG8iID4+IGhhbGxvLnR4dAo=",
                    Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap(),
                ),
            }),
            _ => HttpResponse::InternalServerError().json(err),
        }
    }
}

#[derive(failure::Fail, Debug)]
pub enum Error {
    #[fail(display = "Generic Error {}", _0)]
    General(Box<dyn std::error::Error + Sync + Send>),
    #[fail(display = "Duplicate IliasID")]
    DuplicateIliasId,
    // maybe return the ilias id back
    #[fail(display = "No Results not found for given IliasID: {}", _0)]
    NotFoundIliasId(IliasId),
    #[fail(display = "No Results not found for given AssignmentID: {}", _0)]
    NotAssignment(AssignmentId),
    #[fail(display = "Request body error. {:?}", _0)]
    Body(JsonPayloadError),
    #[fail(display = "Testing Server {} seems to be not reachable", url)]
    RpcOffline { url: String },
    #[fail(display = "Bad request")]
    BadRequest,
    #[fail(display = " Wrong credentials")]
    Unauthorized,
}
impl<T> From<T> for Error
where
    T: std::error::Error + Sync + Send + 'static,
{
    fn from(error: T) -> Self {
        Error::General(Box::new(error))
    }
}

impl Error {
    pub(crate) fn into_actix_web_err(self) -> actix_web::Error {
        actix_web::error::ErrorUnauthorized(self.to_string())
    }
}
