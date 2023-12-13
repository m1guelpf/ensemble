use deluxe::{ParseMetaItem, ParseMode};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{parse::ParseStream, Expr};

use super::field::Fields;

#[derive(Debug, Default)]
pub enum Value {
	#[default]
	Default,
	Expr(Expr),
}

impl ParseMetaItem for Value {
	fn parse_meta_item(input: ParseStream, _mode: ParseMode) -> syn::Result<Self> {
		Ok(Self::Expr(input.parse::<Expr>()?))
	}

	fn parse_meta_item_flag(_: Span) -> syn::Result<Self> {
		Ok(Self::Default)
	}
}

#[derive(Debug, ParseMetaItem, Default)]
#[deluxe(default)]
pub struct Options {
	pub uuid: bool,
	pub created_at: bool,
	pub updated_at: bool,
	pub incrementing: Option<bool>,
	#[deluxe(rename = default)]
	pub value: Option<Value>,
}

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
