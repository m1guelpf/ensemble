use deluxe::ExtractAttributes;
use inflector::Inflector;
use pluralizer::pluralize;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::DeriveInput;

use self::field::{Field, Fields};

mod default;
mod field;
mod serde;

#[derive(ExtractAttributes, Default)]
#[deluxe(attributes(ensemble), default)]
pub struct Opts {
    #[deluxe(rename = table)]
    table_name: Option<String>,
}

pub fn r#impl(ast: &DeriveInput, opts: Opts) -> syn::Result<proc_macro2::TokenStream> {
    let syn::Data::Struct(r#struct) = &ast.data else {
        return Err(syn::Error::new_spanned(
            ast,
            "Model derive only supports structs",
        ));
    };

    let syn::Fields::Named(struct_fields) = &r#struct.fields else {
        return Err(syn::Error::new_spanned(
            ast,
            "Model derive only supports named fields",
        ));
    };

    let fields = Fields::from(struct_fields.clone());
    let primary_key = fields.primary_key()?;

    let keys_impl = impl_keys(&fields);
    let find_impl = impl_find(primary_key);
    let create_impl = impl_create(&fields, primary_key)?;
    let primary_key_impl = impl_primary_key(primary_key);
    let serde_impl = serde::r#impl(&ast.ident, &fields);
    let default_impl = default::r#impl(&ast.ident, &fields)?;
    let table_name_impl = impl_table_name(&ast.ident.to_string(), opts.table_name);

    let name = &ast.ident;
    let primary_key_type = &primary_key.ty;
    let gen = quote! {
        #[automatically_derived]
        #[ensemble::async_trait]
        impl Model for #name {
            type PrimaryKey = #primary_key_type;
            const NAME: &'static str = stringify!(#name);

            #keys_impl
            #find_impl
            #create_impl
            #table_name_impl
            #primary_key_impl
        }
        #serde_impl
        #default_impl
    };

    Ok(gen)
}

fn impl_find(primary_key: &Field) -> TokenStream {
    let ident = &primary_key.ident;

    quote! {
        async fn find(#ident: Self::PrimaryKey) -> Result<Self, ensemble::query::Error> {
            ensemble::query::find(&#ident).await
        }
    }
}

fn impl_create(fields: &Fields, primary_key: &Field) -> syn::Result<TokenStream> {
    let mut required = vec![];

    for field in &fields.fields {
        if field.default()?.is_some() {
            continue;
        }

        let ty = &field.ty;
        let ident = &field.ident;
        required.push(quote_spanned! {field.span() =>
            if self.#ident == <#ty>::default() {
                return Err(ensemble::query::Error::Required(stringify!(#ident)));
            }
        });
    }

    let optional_increment = if primary_key.attr.default.increments {
        let primary_key = &primary_key.ident;
        quote! {
            |(mut model, id)| {
                model.#primary_key = id;

                model
            }
        }
    } else {
        quote! { |(mut model, _)| model }
    };

    Ok(quote! {
        async fn create(self) -> Result<Self, ensemble::query::Error> {
            #(#required)*
            ensemble::query::create(self).await.map(#optional_increment)
        }
    })
}

fn impl_primary_key(primary_key: &Field) -> TokenStream {
    let ident = &primary_key.ident;

    quote! {
        const PRIMARY_KEY: &'static str = stringify!(#ident);

        fn primary_key(&self) -> &Self::PrimaryKey {
            &self.#ident
        }
    }
}

fn impl_keys(fields: &Fields) -> TokenStream {
    let keys = fields.keys();

    quote! {
        fn keys() -> Vec<&'static str> {
            vec![
                #(stringify!(#keys),)*
            ]
        }
    }
}

fn impl_table_name(struct_name: &str, custom_name: Option<String>) -> TokenStream {
    let table_name =
        custom_name.unwrap_or_else(|| pluralize(&struct_name.to_snake_case(), 2, false));

    quote! {
        const TABLE_NAME: &'static str = #table_name;
    }
}
