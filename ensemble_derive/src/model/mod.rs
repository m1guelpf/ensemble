use deluxe::ExtractAttributes;
use inflector::Inflector;
use pluralizer::pluralize;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::DeriveInput;

use crate::Relationship;

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

    let fields = Fields::try_from(struct_fields.clone())?;
    let primary_key = fields.primary_key()?;

    let keys_impl = impl_keys(&fields);
    let find_impl = impl_find(primary_key);
    let fresh_impl = impl_fresh(primary_key);
    let eager_load_impl = impl_eager_load(&fields);
    let save_impl = impl_save(&fields, primary_key);
    let primary_key_impl = impl_primary_key(primary_key);
    let fill_relation_impl = impl_fill_relation(&fields);
    let serde_impl = serde::r#impl(&ast.ident, &fields)?;
    let default_impl = default::r#impl(&ast.ident, &fields)?;
    let create_impl = impl_create(&ast.ident, &fields, primary_key);
    let relationships_impl = impl_relationships(&ast.ident, &fields)?;
    let table_name_impl = impl_table_name(&ast.ident.to_string(), opts.table_name);

    let name = &ast.ident;
    let primary_key_type = &primary_key.ty;
    let gen = quote! {
        const _: () = {
            use ::ensemble::relationships::Relationship;
            #[automatically_derived]
            #[ensemble::async_trait]
            impl Model for #name {
                type PrimaryKey = #primary_key_type;
                const NAME: &'static str = stringify!(#name);

                #save_impl
                #keys_impl
                #find_impl
                #fresh_impl
                #create_impl
                #table_name_impl
                #eager_load_impl
                #primary_key_impl
                #fill_relation_impl
            }
            #serde_impl
            #default_impl
            #relationships_impl
        };
    };

    Ok(gen)
}

fn impl_fill_relation(fields: &Fields) -> TokenStream {
    let relationships = fields.relationships();

    let fill_relation = relationships.iter().map(|field| {
        let ident = &field.ident;

        quote_spanned! {field.span() =>
            stringify!(#ident) => self.#ident.r#match(related),
        }
    });

    quote! {
        fn fill_relation(&mut self, relation: &str, related: &[::std::collections::HashMap<::std::string::String, ::ensemble::rbs::Value>]) -> Result<(), ::ensemble::query::Error> {
            match relation {
                #(#fill_relation)*
                _ => panic!("Model does not have a {relation} relation"),
            }
        }
    }
}
fn impl_eager_load(fields: &Fields) -> TokenStream {
    let relationships = fields.relationships();

    let eager_loads = relationships.iter().map(|field| {
        let ident = &field.ident;

        quote_spanned! {field.span() =>
            stringify!(#ident) => self.#ident.eager_query(related.iter().map(|model| &model.#ident.value).cloned().collect()),
        }
    });

    quote! {
        fn eager_load(&self, relation: &str, related: &[&Self]) -> ::ensemble::builder::Builder {
            match relation {
                #(#eager_loads)*
                _ => panic!("Model does not have a {relation} relation"),
            }
        }
    }
}

fn impl_fresh(primary_key: &Field) -> TokenStream {
    let ident = &primary_key.ident;

    quote! {
        async fn fresh(&self) -> Result<Self, ::ensemble::query::Error> {
            Self::find(self.#ident.clone()).await
        }
    }
}

fn impl_relationships(name: &Ident, fields: &Fields) -> syn::Result<TokenStream> {
    let primary_key = fields.primary_key()?;
    let relationships = fields.relationships();

    if relationships.is_empty() {
        return Ok(TokenStream::new());
    }

    let impls = relationships.iter().map(|f| {
        let ident = &f.ident;
        let (r#type, related, _) = f.relationship(primary_key).unwrap();
        let return_type = match r#type {
            Relationship::HasMany | Relationship::BelongsToMany => {
                quote! { ::std::vec::Vec<#related> }
            }
            Relationship::HasOne | Relationship::BelongsTo => {
                quote! { #related }
            }
        };

        quote_spanned! {f.span() =>
            pub async fn #ident(&mut self) -> Result<&#return_type, ::ensemble::query::Error> {
                self.#ident.get().await
            }
        }
    });

    Ok(quote! {
        impl #name {
            #(#impls)*
        }
    })
}

fn impl_save(fields: &Fields, primary_key: &Field) -> TokenStream {
    let ident = &primary_key.ident;
    let run_validation = if fields.should_validate() {
        quote! {
            self.validate()?;
        }
    } else {
        TokenStream::new()
    };
    let update_timestamp = fields
        .fields
        .iter()
        .filter(|f| f.attr.default.updated_at)
        .map(|field| {
            let ident = &field.ident;

            quote_spanned! {field.span() =>
                self.#ident = ::ensemble::types::DateTime::now();
            }
        })
        .collect::<TokenStream>();

    quote! {
        async fn save(&mut self) -> Result<(), ::ensemble::query::Error> {
            #update_timestamp
            #run_validation

            let rows_affected = Self::query()
                .r#where(Self::PRIMARY_KEY, "=", &self.#ident)
                .update(::ensemble::value::for_db(self)?)
                .await?;

            if rows_affected != 1 {
                return Err(::ensemble::query::Error::UniqueViolation);
            }

            Ok(())
        }
    }
}

fn impl_find(primary_key: &Field) -> TokenStream {
    let ident = &primary_key.ident;

    quote! {
        async fn find(#ident: Self::PrimaryKey) -> Result<Self, ::ensemble::query::Error> {
            Self::query()
                .r#where(Self::PRIMARY_KEY, "=", ::ensemble::value::for_db(#ident)?)
                .first()
                .await?
                .ok_or(::ensemble::query::Error::NotFound)
        }
    }
}

fn impl_create(name: &Ident, fields: &Fields, primary_key: &Field) -> TokenStream {
    let is_primary_u64 = (&primary_key.ty).into_token_stream().to_string() == "u64";

    let required = fields
        .fields
        .iter()
        .filter(|f| {
            f.default(name, primary_key)
                .map(|o| o.is_none())
                .unwrap_or(false)
        })
        .map(|field| {
            let ty = &field.ty;
            let ident = &field.ident;

            quote_spanned! {field.span() =>
                if self.#ident == <#ty>::default() {
                    return Err(::ensemble::query::Error::Required(stringify!(#ident)));
                }
            }
        });

    let run_validation = if fields.should_validate() {
        quote! {
            self.validate()?;
        }
    } else {
        TokenStream::new()
    };

    let update_timestamps = fields
        .fields
        .iter()
        .filter(|f| f.attr.default.created_at || f.attr.default.updated_at)
        .map(|field| {
            let ident = &field.ident;

            quote_spanned! {field.span() =>
                self.#ident = ::ensemble::types::DateTime::now();
            }
        });

    let insert_and_return = if primary_key
        .attr
        .default
        .incrementing
        .unwrap_or(is_primary_u64)
    {
        let primary_key = &primary_key.ident;
        quote! {
            self.#primary_key = Self::query().insert(::ensemble::value::for_db(&self)?).await?;

            Ok(self)
        }
    } else {
        quote! {
            Self::query().insert(::ensemble::value::for_db(&self)?).await?;

            Ok(self)
        }
    };

    quote! {
        async fn create(mut self) -> Result<Self, ::ensemble::query::Error> {
            #(#update_timestamps)*
            #run_validation
            #(#required)*
            #insert_and_return
        }
    }
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
