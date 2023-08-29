use std::{collections::HashMap, rc::Rc};

use deluxe::ExtractAttributes;
use inflector::Inflector;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{spanned::Spanned, FieldsNamed, GenericArgument, PathArguments, Type};

use crate::Relationship;

use super::default;

pub struct Fields {
    ast: FieldsNamed,
    pub fields: Vec<Field>,
}

pub struct Field {
    pub attr: Attr,
    ast: syn::Field,
    pub ty: syn::Type,
    pub ident: syn::Ident,
    pub has_validation: bool,
}

#[derive(Debug, ExtractAttributes, Default)]
#[deluxe(attributes(validate), default)]
struct ValidationAttr {
    #[deluxe(rest)]
    rules: HashMap<syn::Path, syn::Expr>,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(ExtractAttributes, Default)]
#[deluxe(attributes(model), default)]
pub struct Attr {
    #[cfg(feature = "json")]
    pub hide: bool,
    #[cfg(feature = "json")]
    pub show: bool,
    pub primary: bool,
    pub column: Option<String>,
    pub local_key: Option<String>,
    pub foreign_key: Option<String>,
    pub pivot_table: Option<String>,

    #[deluxe(flatten)]
    pub default: default::Options,
    #[deluxe(skip)]
    pub used_in_relationship: bool,
}

impl Field {
    pub fn new(mut field: syn::Field) -> Self {
        let ident = field.ident.clone().unwrap();
        let mut attr = Attr::extract_attributes(&mut field.attrs).unwrap();
        let validation = ValidationAttr::extract_attributes(&mut field.attrs).unwrap();

        #[cfg(feature = "json")]
        {
            attr.hide |= ident == "password";
        }
        attr.default.created_at |= ident == "created_at";
        attr.default.updated_at |= ident == "updated_at";

        Self {
            attr,
            ident,
            ty: field.ty.clone(),
            ast: field,
            has_validation: !validation.rules.is_empty(),
        }
    }

    pub fn span(&self) -> proc_macro2::Span {
        self.ast.span()
    }

    pub fn default(&self, name: &Ident, primary_key: &Self) -> syn::Result<Option<TokenStream>> {
        let attrs = &self.attr.default;
        let is_primary = primary_key.ident == self.ident;
        let is_u64 = self.ty.to_token_stream().to_string() == "u64";

        Ok(if let Some(default) = &attrs.value {
            Some(quote_spanned! { self.span() => #default })
        } else if attrs.uuid {
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

            Some(quote_spanned! { self.span() => <#ty>::new_v4() })
        } else if attrs.incrementing.unwrap_or(is_primary && is_u64) {
            Some(quote_spanned! { self.span() => 0 })
        } else if attrs.created_at || attrs.updated_at {
            let Type::Path(ty) = &self.ty else {
                return Err(syn::Error::new_spanned(
                    &self.ty,
                    "Field must be of type ensemble::types::DateTime",
                ));
            };

            Some(quote_spanned! { self.span() => <#ty>::now() })
        } else if let Some((relationship_type, related, _)) = self.relationship(primary_key) {
            let relationship_ident = Ident::new(&relationship_type.to_string(), self.span());
            let foreign_key = self.foreign_key(relationship_type);

            if self.attr.column == Some(self.ident.to_string()) {
                return Err(syn::Error::new_spanned(
                    self,
                    "You cannot name a relationship field the same as the column it references.",
                ));
            }

            Some(
                quote_spanned! { self.span() => <#relationship_ident<#name, #related>>::build(Default::default(), None, #foreign_key) },
            )
        } else {
            None
        })
    }

    pub(crate) fn foreign_key(&self, relationship_type: Relationship) -> TokenStream {
        match relationship_type {
            Relationship::BelongsToMany => {
                let local_key = wrap_option(self.attr.local_key.clone());
                let pivot_table = wrap_option(self.attr.pivot_table.clone());
                let foreign_key = wrap_option(self.attr.foreign_key.clone());

                quote_spanned! {self.span()=> (#pivot_table, #foreign_key, #local_key) }
            }
            _ => wrap_option(self.attr.foreign_key.clone()),
        }
    }

    pub fn has_relationship(&self) -> bool {
        let Type::Path(ty) = &self.ty else {
            return false;
        };

        let Some(ty) = ty.path.segments.first() else {
            return false;
        };

        ["HasOne", "HasMany", "BelongsTo", "BelongsToMany"].contains(&ty.ident.to_string().as_str())
    }

    pub(crate) fn relationship(&self, primary_key: &Self) -> Option<(Relationship, Ident, String)> {
        let Type::Path(ty) = &self.ty else {
            return None;
        };

        let Some(ty) = ty.path.segments.first() else {
            return None;
        };

        let relationship_type = ty.ident.to_string();
        if !["HasOne", "HasMany", "BelongsTo", "BelongsToMany"]
            .contains(&relationship_type.as_str())
        {
            return None;
        }
        let relationship_type: Relationship = relationship_type.into();

        let PathArguments::AngleBracketed(ty) = &ty.arguments else {
            panic!("Expected generic argument");
        };
        let GenericArgument::Type(Type::Path(ty)) = ty.args.last().unwrap() else {
            panic!("Expected generic argument");
        };

        let related = &ty.path.segments.first().unwrap().ident;

        let value_key = match relationship_type {
            Relationship::BelongsToMany | Relationship::HasOne | Relationship::HasMany => {
                primary_key.ident.to_string()
            }
            Relationship::BelongsTo => self
                .attr
                .column
                .clone()
                .unwrap_or_else(|| related.to_string().to_foreign_key()),
        };

        Some((relationship_type, related.clone(), value_key))
    }
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ast.to_tokens(tokens);
    }
}

impl Fields {
    pub fn should_validate(&self) -> bool {
        self.fields.iter().any(|f| f.has_validation)
    }

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

    pub fn relationships(&self) -> Vec<&Field> {
        self.fields
            .iter()
            .filter(|f| f.has_relationship())
            .collect()
    }

    pub fn mark_relationship_keys(&mut self) -> syn::Result<()> {
        let primary_key = self.primary_key()?;
        let relationship_keys = self
            .relationships()
            .iter()
            .filter_map(|f| f.relationship(primary_key))
            .map(|(_, _, key)| key)
            .collect::<Rc<_>>();

        self.fields
            .iter_mut()
            .filter(|f| relationship_keys.contains(&f.ident.to_string()))
            .for_each(|f| {
                f.attr.used_in_relationship = true;
            });

        Ok(())
    }
}

impl ToTokens for Fields {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ast.to_tokens(tokens);
    }
}

impl TryFrom<FieldsNamed> for Fields {
    type Error = syn::Error;
    fn try_from(ast: FieldsNamed) -> Result<Self, Self::Error> {
        let fields = ast.named.iter().map(|f| Field::new(f.clone())).collect();

        let mut fields = Self { ast, fields };

        fields.mark_relationship_keys()?;

        Ok(fields)
    }
}

fn wrap_option<T: quote::ToTokens>(option: Option<T>) -> TokenStream {
    option.map_or_else(
        || quote! { None },
        |value| quote! { Some(#value.to_string()) },
    )
}
