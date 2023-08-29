use std::rc::Rc;

use crate::{
    connection::{self, ConnectError},
    value, Model,
};

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
}

/// Insert a new model into the database.
///
/// # Errors
///
/// Returns an error if the model cannot be inserted, or if a connection to the database cannot be established.
pub async fn create<M: Model>(model: M) -> Result<(M, M::PrimaryKey), Error> {
    let mut conn = connection::get().await?;

    let result = conn
        .exec(
            &format!(
                "INSERT INTO {} ({}) VALUES ({})",
                M::TABLE_NAME,
                M::keys().join(", "),
                M::keys()
                    .into_iter()
                    .map(|_| "?")
                    .collect::<Rc<_>>()
                    .join(", ")
            ),
            value::into(&model),
        )
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

    if result.rows_affected != 1 {
        return Err(Error::Database(format!(
            "Expected to update 1 row, updated {}",
            result.rows_affected
        )));
    }

    Ok((model, rbs::from_value(result.last_insert_id)?))
}
