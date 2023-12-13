#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::fmt::Display;
use syn::{parse_macro_input, DeriveInput};

mod column;
mod model;

#[proc_macro_derive(Model, attributes(ensemble, model, validate))]
pub fn derive_model(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let mut ast = parse_macro_input!(input as DeriveInput);
	let opts = match deluxe::extract_attributes(&mut ast) {
		Ok(opts) => opts,
		Err(e) => return e.into_compile_error().into(),
	};

	model::r#impl(&ast, opts)
		.unwrap_or_else(syn::Error::into_compile_error)
		.into()
}

#[proc_macro_derive(Column, attributes(builder))]
pub fn derive_column(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let ast = parse_macro_input!(input as DeriveInput);

	column::r#impl(&ast)
		.unwrap_or_else(syn::Error::into_compile_error)
		.into()
}

#[derive(Clone, Copy)]
pub(crate) enum Relationship {
	HasOne,
	HasMany,
	BelongsTo,
	BelongsToMany,
}

impl Display for Relationship {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::HasOne => "HasOne",
				Self::HasMany => "HasMany",
				Self::BelongsTo => "BelongsTo",
				Self::BelongsToMany => "BelongsToMany",
			}
		)
	}
}

#[allow(clippy::fallible_impl_from)]
impl From<String> for Relationship {
	fn from(value: String) -> Self {
		match value.as_str() {
			"HasOne" => Self::HasOne,
			"HasMany" => Self::HasMany,
			"BelongsTo" => Self::BelongsTo,
			"BelongsToMany" => Self::BelongsToMany,
			_ => panic!("Unknown relationship found."),
		}
	}
}
