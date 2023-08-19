use crate::{
    connection::{self, ConnectError},
    Model,
};
use rbs::to_value;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Connection(#[from] ConnectError),

    #[error(transparent)]
    Database(#[from] rbatis::Error),

    #[error("Failed to serialize model.")]
    Serialization(#[from] rbs::value::ext::Error),

    #[error("The model could not be found.")]
    NotFound,

    #[error("The unique constraint was violated.")]
    UniqueViolation,
}

/// Get all of the models from the database.
///
/// # Errors
///
/// Returns an error if the query fails, or if a connection to the database cannot be established.
pub async fn all<M: Model>() -> Result<Vec<M>, Error> {
    let mut conn = connection::get().await?;

    let result = conn
        .get_values(&format!("SELECT * FROM {}", M::TABLE_NAME), vec![])
        .await?;

    Ok(result
        .into_iter()
        .map(rbs::from_value)
        .collect::<Result<_, _>>()?)
}

/// Find a model by its primary key.
///
/// # Errors
///
/// Returns an error if the model cannot be found, or if a connection to the database cannot be established.
pub async fn find<M: Model>(key: M::PrimaryKey) -> Result<M, Error> {
    let mut conn = connection::get().await?;

    let result = conn
        .get_values(
            &format!("SELECT * FROM {} WHERE `id` = ?", M::TABLE_NAME),
            vec![to_value!(key)],
        )
        .await?;

    match result.len() {
        0 => Err(Error::NotFound),
        1 => Ok(rbs::from_value(
            result.into_iter().next().unwrap_or_else(|| unreachable!()),
        )?),
        _ => Err(Error::UniqueViolation),
    }
}
