use deluxe::ExtractAttributes;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote_spanned, ToTokens};
use syn::{spanned::Spanned, FieldsNamed, Type};

use super::default;

pub struct Fields {
    ast: FieldsNamed,
    pub fields: Vec<Field>,
}

pub struct Field {
    pub attr: Attr,
    pub ty: syn::Type,
    pub ident: syn::Ident,
    ast: syn::Field,
}

#[derive(ExtractAttributes, Default)]
#[deluxe(attributes(model), default)]
pub struct Attr {
    pub primary: bool,
    #[deluxe(flatten)]
    pub default: default::Options,
}

impl Field {
    pub fn new(mut field: syn::Field) -> Self {
        let ident = field.ident.clone().unwrap();
        let mut attr = Attr::extract_attributes(&mut field.attrs).unwrap();

        attr.default.created_at |= ident == "created_at";
        attr.default.updated_at |= ident == "updated_at";

        Self {
            attr,
            ident,
            ty: field.ty.clone(),
            ast: field,
        }
    }

    pub fn span(&self) -> proc_macro2::Span {
        self.ast.span()
    }

    pub fn default(&self) -> syn::Result<Option<TokenStream>> {
        let attrs = &self.attr.default;

        Ok(if let Some(default) = &attrs.value {
            Some(quote_spanned! { self.span() => #default })
        } else if let Some(uuid) = attrs.uuid.version() {
            let Type::Path(ty) = &self.ty else {
                return Err(syn::Error::new_spanned(
                    self,
                    "Field must be of type uuid::Uuid",
                ));
            };

            if ty.path.segments.last().unwrap().ident != "Uuid" {
                return Err(syn::Error::new_spanned(
                    ty,
                    "Field must be of type uuid::Uuid",
                ));
            }

            let new_fn = format_ident!("new_{uuid}");
            Some(quote_spanned! { self.span() => <#ty>::#new_fn() })
        } else if attrs.increments {
            Some(quote_spanned! { self.span() => 0 })
        } else if attrs.created_at || attrs.updated_at {
            let Type::Path(ty) = &self.ty else {
                return Err(syn::Error::new_spanned(
                    &self.ty,
                    "Field must be of type ensemble::types::DateTime",
                ));
            };

            Some(quote_spanned! { self.span() => <#ty>::now() })
        } else {
            None
        })
    }
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ast.to_tokens(tokens);
    }
}

impl Fields {
    pub fn primary_key(&self) -> syn::Result<&Field> {
        let mut primary = None;
        let mut id_field = None;

        for field in &self.fields {
            if field.attr.primary {
                if primary.is_some() {
                    return Err(syn::Error::new_spanned(
                        field,
                        "Only one field can be marked as primary",
                    ));
                }

                primary = Some(field);
            } else if field.ident == "id" {
                id_field = Some(field);
            }
        }

        primary.or(id_field).ok_or_else(|| {
            syn::Error::new_spanned(
            self,
            "No primary key found. Either mark a field with `#[model(primary)]` or name it `id`.",
            )
        })
    }

    pub fn keys(&self) -> Vec<&Ident> {
        let mut keys = vec![];

        for field in &self.fields {
            keys.push(&field.ident);
        }

        keys
    }
}

impl ToTokens for Fields {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ast.to_tokens(tokens);
    }
}

impl From<FieldsNamed> for Fields {
    fn from(ast: FieldsNamed) -> Self {
        let fields = ast.named.iter().map(|f| Field::new(f.clone())).collect();

        Self { ast, fields }
    }
}
