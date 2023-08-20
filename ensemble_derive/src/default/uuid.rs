use deluxe::{ParseMetaItem, ParseMode};
use proc_macro2::Span;
use syn::parse::ParseStream;

#[derive(Debug, Default)]
pub enum Version {
    #[default]
    None,
    Default,
    Version(String),
}

impl Version {
    pub fn version(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::Default => Some("v4"),
            Self::Version(ver) => Some(ver),
        }
    }
}

impl ParseMetaItem for Version {
    fn parse_meta_item(input: ParseStream, _mode: ParseMode) -> syn::Result<Self> {
        let version = input.parse::<syn::LitStr>()?;

        Ok(Self::Version(version.value()))
    }

    fn parse_meta_item_flag(_: Span) -> syn::Result<Self> {
        Ok(Self::Default)
    }
}
