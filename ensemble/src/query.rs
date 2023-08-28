use std::rc::Rc;

use crate::{
    connection::{self, ConnectError},
    value, Model,
};
use rbs::to_value;

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

/// Get all of the models from the database.
///
/// # Errors
///
/// Returns an error if the query fails, or if a connection to the database cannot be established.
pub async fn all<M: Model>() -> Result<Vec<M>, Error> {
    let mut conn = connection::get().await?;

    let result = conn
        .get_values(&format!("SELECT * FROM {}", M::TABLE_NAME), vec![])
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

    Ok(result
        .into_iter()
        .map(value::from)
        .collect::<Result<_, _>>()?)
}

/// Find a model by its primary key.
///
/// # Errors
///
/// Returns an error if the model cannot be found, or if a connection to the database cannot be established.
pub async fn find<M: Model>(key: &M::PrimaryKey) -> Result<M, Error> {
    let mut conn = connection::get().await?;

    let result = conn
        .get_values(
            &format!(
                "SELECT * FROM {} WHERE {} = ?",
                M::TABLE_NAME,
                M::PRIMARY_KEY
            ),
            vec![to_value!(key)],
        )
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

    match result.len() {
        0 => Err(Error::NotFound),
        1 => Ok(value::from(
            result.into_iter().next().unwrap_or_else(|| unreachable!()),
        )?),
        _ => Err(Error::UniqueViolation),
    }
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

/// Update the model in the database.
///
/// # Errors
///
/// Returns an error if the model cannot be updated, or if a connection to the database cannot be established.
pub async fn save<M: Model>(model: &M) -> Result<(), Error> {
    let mut conn = connection::get().await?;

    let result = conn
        .exec(
            &format!(
                "UPDATE {} SET {} WHERE {} = {}",
                M::TABLE_NAME,
                M::keys()
                    .into_iter()
                    .map(|key| format!("{key} = ?"))
                    .collect::<Rc<_>>()
                    .join(", "),
                M::PRIMARY_KEY,
                model.primary_key()
            ),
            value::into(model),
        )
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

    if result.rows_affected == 1 {
        Ok(())
    } else {
        Err(Error::Database(format!(
            "Expected to update 1 row, updated {}",
            result.rows_affected
        )))
    }
}

/// Delete the model from the database.
///
/// # Errors
///
/// Returns an error if the model cannot be deleted, or if a connection to the database cannot be established.
pub async fn delete<M: Model>(model: &M) -> Result<(), Error> {
    let mut conn = connection::get().await?;

    let result = conn
        .exec(
            &format!(
                "DELETE FROM {} WHERE {} = {}",
                M::TABLE_NAME,
                M::PRIMARY_KEY,
                model.primary_key()
            ),
            vec![],
        )
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

    if result.rows_affected == 1 {
        Ok(())
    } else {
        Err(Error::Database(format!(
            "Expected to affect 1 row, affect {}",
            result.rows_affected
        )))
    }
}
