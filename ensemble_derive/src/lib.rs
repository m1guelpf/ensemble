use deluxe::{ExtractAttributes, ParseMetaItem, ParseMode};
use pluralizer::pluralize;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    parse::ParseStream, parse_macro_input, spanned::Spanned, DeriveInput, FieldsNamed, LitStr,
    PathArguments, Type,
};

#[derive(Debug, Default)]
enum UuidVersion {
    #[default]
    None,
    Default,
    Version(String),
}

impl UuidVersion {
    fn version(self) -> Option<String> {
        match self {
            UuidVersion::None => None,
            UuidVersion::Default => Some("v4".to_string()),
            UuidVersion::Version(ver) => Some(ver),
        }
    }
}

impl ParseMetaItem for UuidVersion {
    fn parse_meta_item(input: ParseStream, _mode: ParseMode) -> syn::Result<Self> {
        let version = input.parse::<syn::LitStr>()?;

        Ok(Self::Version(version.value()))
    }

    fn parse_meta_item_flag(_: Span) -> syn::Result<Self> {
        Ok(Self::Default)
    }
}

#[derive(ExtractAttributes, Default)]
#[deluxe(attributes(ensemble), default)]
struct Opts {
    table_name: Option<String>,
}

#[derive(ExtractAttributes, Default)]
#[deluxe(attributes(model), default)]
struct Field {
    primary: bool,
    created_at: bool,
    updated_at: bool,
    #[deluxe(default)]
    uuid: UuidVersion,
    default: Option<LitStr>,
}

#[proc_macro_derive(Model, attributes(ensemble, model))]
pub fn derive_model(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let opts = match deluxe::extract_attributes(&mut ast) {
        Ok(opts) => opts,
        Err(e) => return e.into_compile_error().into(),
    };

    impl_model(&ast, opts)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn impl_model(ast: &DeriveInput, opts: Opts) -> syn::Result<proc_macro2::TokenStream> {
    let r#struct = match &ast.data {
        syn::Data::Struct(s) => s,
        _ => {
            return Err(syn::Error::new_spanned(
                ast,
                "Model derive only supports structs",
            ))
        }
    };

    let struct_fields = match &r#struct.fields {
        syn::Fields::Named(f) => f,
        _ => {
            return Err(syn::Error::new_spanned(
                ast,
                "Model derive only supports named fields",
            ))
        }
    };

    let primary_key = find_primary_key(struct_fields)?;
    let primary_key_type = &primary_key.ty;

    let find_impl = impl_find(&primary_key);
    let keys_impl = impl_keys(struct_fields)?;
    let default_impl = impl_default(struct_fields)?;
    let primary_key_impl = impl_primary_key(&primary_key)?;
    let table_name_impl = impl_table_name(&ast.ident.to_string(), opts.table_name);

    let name = &ast.ident;
    let gen = quote! {
        impl Model for #name {
            type PrimaryKey = #primary_key_type;

            #keys_impl
            #find_impl
            #primary_key_impl
            #table_name_impl
        }
        impl core::default::Default for #name {
            #default_impl
        }
    };

    Ok(gen)
}

fn impl_find(primary_key: &syn::Field) -> TokenStream {
    let primary_type = &primary_key.ty;

    quote! {
        fn find(id: #primary_type) -> Result<Self, ensemble::FindError> {
            Err(ensemble::FindError::Unimplemented)
        }
    }
}

fn find_primary_key(ast: &FieldsNamed) -> syn::Result<syn::Field> {
    let mut id_field = None;
    let mut primary = None;
    let mut uuid_field = None;

    for field in &ast.named {
        let attrs = Field::extract_attributes(&mut field.attrs.clone())?;

        if attrs.primary {
            if primary.is_some() {
                return Err(syn::Error::new_spanned(
                    field,
                    "Only one field can be marked as primary",
                ));
            }

            primary = Some(field);
        } else if field.ident.as_ref().unwrap() == "id" {
            id_field = Some(field);
        } else if field.ident.as_ref().unwrap() == "uuid" {
            uuid_field = Some(field);
        }
    }

    primary.or(id_field).or(uuid_field).ok_or_else(|| {
        syn::Error::new_spanned(
            ast,
            "No primary key found. Either mark a field with `#[model(primary)]` or name a field `id` or `uuid`",
        )
    }).cloned()
}

fn impl_primary_key(primary: &syn::Field) -> syn::Result<TokenStream> {
    let ident = primary.ident.clone().unwrap();

    Ok(quote! {
        fn primary_key() -> &'static str {
            stringify!(#ident)
        }
    })
}

fn impl_default(ast: &FieldsNamed) -> syn::Result<TokenStream> {
    let mut defaults: Vec<TokenStream> = vec![];

    for field in &ast.named {
        let ident = field.ident.clone().unwrap();
        let attrs = Field::extract_attributes(&mut field.attrs.clone())?;

        defaults.push(if let Some(default) = attrs.default {
            match default.parse::<syn::Expr>() {
                Ok(expr) => quote_spanned! { field.span() => #ident: #expr },
                Err(_) => return Err(syn::Error::new_spanned(field, "Invalid default expression")),
            }
        } else if let Some(uuid) = attrs.uuid.version() {
            let Type::Path(ty) = &field.ty else {
                return Err(syn::Error::new_spanned(
                    field,
                    "Field must be of type uuid::Uuid",
                ));
            };

            let new_fn = format_ident!("new_{uuid}");
            quote_spanned! { field.span() => #ident: #ty::#new_fn() }
        } else if attrs.created_at || attrs.updated_at {
            let Type::Path(ty) = &field.ty else {
                return Err(syn::Error::new_spanned(
                    field,
                    "Field must be of type chrono::DateTime<TimeZone>",
                ));
            };

            let ty = match &ty.path.segments.last().unwrap().arguments {
                PathArguments::AngleBracketed(args) => match args.args.first().unwrap() {
                    syn::GenericArgument::Type(ty) => ty,
                    _ => {
                        return Err(syn::Error::new_spanned(
                            field,
                            "Field must be of type chrono::DateTime<TimeZone>",
                        ))
                    }
                },
                _ => {
                    return Err(syn::Error::new_spanned(
                        field,
                        "Field must be of type chrono::DateTime<TimeZone>",
                    ))
                }
            };

            quote_spanned! { field.span() => #ident: #ty::now() }
        } else {
            quote_spanned! { field.span() => #ident: Default::default() }
        });
    }

    Ok(quote! {
        fn default() -> Self {
            Self {
                #(#defaults,)*
            }
        }
    })
}

fn impl_keys(ast: &FieldsNamed) -> syn::Result<TokenStream> {
    let mut keys = vec![];

    for field in &ast.named {
        let ident = field.ident.clone().unwrap();
        keys.push(ident);
    }

    Ok(quote! {
        fn keys() -> Vec<&'static str> {
            vec![
                #(stringify!(#keys),)*
            ]
        }
    })
}

fn impl_table_name(struct_name: &str, custom_name: Option<String>) -> TokenStream {
    let table_name = custom_name.unwrap_or(pluralize(&struct_name.to_lowercase(), 2, false));

    quote! {
        fn table_name() -> &'static str {
            #table_name
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use ensemble::Model;

    #[test]
    fn derives_table_name_from_model_name() {
        #[derive(Model)]
        #[allow(dead_code)]
        struct User {
            id: u8,
        }

        #[derive(Model)]
        #[allow(dead_code)]
        struct Music {
            id: u8,
        }

        #[derive(Model)]
        #[allow(dead_code)]
        struct Index {
            id: u8,
        }

        assert_eq!(User::table_name(), "users");
        assert_eq!(Music::table_name(), "music");
        assert_eq!(Index::table_name(), "indices");
    }

    #[test]
    fn derived_table_name_can_be_overriden_with_attribute() {
        #[derive(Model)]
        #[allow(dead_code)]
        #[ensemble(table_name = "custom_table")]
        struct ModelWithCustomTableName {
            id: u8,
        }

        assert_eq!(ModelWithCustomTableName::table_name(), "custom_table");
    }

    #[test]
    fn keys_extracted() {
        #[derive(Debug, Model)]
        #[allow(dead_code)]
        struct MyModel {
            id: u8,
            name: String,
            email: String,
        }

        assert_eq!(MyModel::keys(), vec!["id", "name", "email",]);
    }

    #[test]
    fn default_impl() {
        #[derive(Debug, Model)]
        pub struct MyModel {
            pub id: u8,
            pub name: String,
            pub created_at: DateTime<Utc>,
            pub updated_at: DateTime<Utc>,
        }

        let user = MyModel::default();
        assert_eq!(user.id, 0);
        assert_eq!(user.name, String::default());
        assert!(user.created_at < Utc::now());
        assert!(user.updated_at < Utc::now());
    }
}
