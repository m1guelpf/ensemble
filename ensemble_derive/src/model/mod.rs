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

    #[cfg(feature = "json")]
    let impl_json = impl_json(&fields);
    #[cfg(not(feature = "json"))]
    let impl_json = TokenStream::new();

    let keys_impl = impl_keys(&fields);
    let find_impl = impl_find(primary_key);
    let save_impl = impl_save(fields.should_validate());
    let primary_key_impl = impl_primary_key(primary_key);
    let serde_impl = serde::r#impl(&ast.ident, &fields);
    let default_impl = default::r#impl(&ast.ident, &fields)?;
    let create_impl = impl_create(&ast.ident, &fields, primary_key)?;
    let table_name_impl = impl_table_name(&ast.ident.to_string(), opts.table_name);

    let name = &ast.ident;
    let primary_key_type = &primary_key.ty;
    let gen = quote! {
        #[automatically_derived]
        #[ensemble::async_trait]
        impl Model for #name {
            type PrimaryKey = #primary_key_type;
            const NAME: &'static str = stringify!(#name);

            #impl_json
            #save_impl
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

#[cfg(feature = "json")]
fn impl_json(fields: &Fields) -> TokenStream {
    let remove_fields = fields
        .fields
        .iter()
        .filter(|field| field.attr.hide && !field.attr.show)
        .map(|f| {
            let ident = &f.ident;
            quote_spanned! {f.span() => value.remove(stringify!(#ident)); }
        });

    quote! {
        fn json(&self) -> ::ensemble::serde_json::Value {
            let value = ::ensemble::serde_json::to_value(self).unwrap();
            let ::ensemble::serde_json::Value::Object(mut value) = value else {
                return value;
            };

            #(#remove_fields)*

            ::ensemble::serde_json::Value::Object(value)
        }
    }
}

fn impl_save(should_validate: bool) -> TokenStream {
    let run_validation = if should_validate {
        quote! {
            self.validate()?;
        }
    } else {
        TokenStream::new()
    };

    quote! {
        async fn save(&mut self) -> Result<(), ::ensemble::query::Error> {
            #run_validation
            ::ensemble::query::save(self).await
        }
    }
}

fn impl_find(primary_key: &Field) -> TokenStream {
    let ident = &primary_key.ident;

    quote! {
        async fn find(#ident: Self::PrimaryKey) -> Result<Self, ::ensemble::query::Error> {
            ::ensemble::query::find(&#ident).await
        }
    }
}

fn impl_create(name: &Ident, fields: &Fields, primary_key: &Field) -> syn::Result<TokenStream> {
    let mut required = vec![];

    for field in &fields.fields {
        if field.default(name, primary_key)?.is_some() {
            continue;
        }

        let ty = &field.ty;
        let ident = &field.ident;
        required.push(quote_spanned! {field.span() =>
            if self.#ident == <#ty>::default() {
                return Err(::ensemble::query::Error::Required(stringify!(#ident)));
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

    let run_validation = if fields.should_validate() {
        quote! {
            self.validate()?;
        }
    } else {
        TokenStream::new()
    };

    Ok(quote! {
        async fn create(self) -> Result<Self, ::ensemble::query::Error> {
            #run_validation
            #(#required)*
            ::ensemble::query::create(self).await.map(#optional_increment)
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
