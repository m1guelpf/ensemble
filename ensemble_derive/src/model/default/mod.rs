use deluxe::ParseMetaItem;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::Expr;

use super::field::Fields;

#[derive(Debug, ParseMetaItem, Default)]
#[deluxe(default)]
pub struct Options {
    pub increments: bool,
    pub created_at: bool,
    pub updated_at: bool,
    #[deluxe(rename = default)]
    pub value: Option<Expr>,
    pub uuid: uuid::Version,
}

pub mod uuid;

pub fn r#impl(name: &Ident, fields: &Fields) -> syn::Result<TokenStream> {
    let mut defaults = vec![];
    let primary_key = fields.primary_key()?;

    for field in &fields.fields {
        let ident = &field.ident;
        let default = field
            .default(name, primary_key)?
            .unwrap_or_else(|| quote_spanned! { field.span() => Default::default() });

        defaults.push(quote_spanned! { field.span() => #ident: #default });
    }

    Ok(quote! {
        #[automatically_derived]
        impl core::default::Default for #name {
            fn default() -> Self {
                Self {
                    #(#defaults,)*
                }
            }
        }
    })
}
