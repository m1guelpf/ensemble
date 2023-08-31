use crate::connection::ConnectError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Connection(#[from] ConnectError),

    #[cfg(feature = "validator")]
    #[error(transparent)]
    Validation(#[from] validator::ValidationErrors),

    #[error("{0}")]
    Database(String),

    #[error("The {0} field is required.")]
    Required(&'static str),

    #[error("Failed to serialize model.")]
    Serialization(#[from] rbs::value::ext::Error),

    #[error("The model could not be found.")]
    NotFound,

    #[error("The unique constraint was violated.")]
    UniqueViolation,

    #[error("The query is invalid.")]
    InvalidQuery,
}
