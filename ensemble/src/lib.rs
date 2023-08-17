pub use ensemble_derive::Model;

#[derive(Debug, thiserror::Error)]
pub enum FindError {
    #[error("This method is not implemented")]
    Unimplemented,
}

pub trait Model {
    type PrimaryKey;

    fn keys() -> Vec<&'static str>;
    fn table_name() -> &'static str;
    fn primary_key() -> &'static str;
    fn find(id: Self::PrimaryKey) -> Result<Self, FindError>
    where
        Self: std::marker::Sized;
}
