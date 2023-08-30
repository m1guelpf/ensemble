use std::rc::Rc;

use inflector::Inflector;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};

use super::field::Fields;

pub fn r#impl(name: &Ident, fields: &Fields) -> TokenStream {
    let mut serde = impl_serialize(name, fields);
    serde.extend(impl_deserialize(name, fields));

    serde
}

pub fn impl_serialize(name: &Ident, fields: &Fields) -> TokenStream {
    let count = fields.fields.len();
    let serialize_field = fields.fields.iter().map(|field| {
        let ident = &field.ident;
        let column = field
            .attr
            .column
            .as_ref()
            .map_or(field.ident.clone(), |v| Ident::new(v, field.span()));

        quote_spanned! {field.span()=>
            state.serialize_field(stringify!(#column), &self.#ident)?;
        }
    });

    #[cfg(feature = "json")]
    let serialize_fields = if fields.fields.iter().any(|f| f.attr.hide && !f.attr.show) {
        let fields_with_hidden = fields.fields.iter().filter_map(|field| {
            if field.attr.hide && !field.attr.show {
                return None;
            }

            let ident = &field.ident;
            let column = field
                .attr
                .column
                .as_ref()
                .map_or(field.ident.clone(), |v| Ident::new(v, field.span()));

            Some(quote_spanned! {field.span()=>
                state.serialize_field(stringify!(#column), &self.#ident)?;
            })
        });

        quote! {
            // ugly hack to figure out if we're serializing for rbs. might break in future (or previous) versions of rust.
            if ::std::any::type_name::<S::Error>() == ::std::any::type_name::<::ensemble::rbs::Error>() {
                #(#serialize_field)*
            } else {
                #(#fields_with_hidden)*
            }
        }
    } else {
        quote! {
            #(#serialize_field)*
        }
    };
    #[cfg(not(feature = "json"))]
    let serialize_fields: TokenStream = quote! {
        #(#serialize_field)*
    };

    quote! {
        const _: () = {
            use ::ensemble::serde::ser::SerializeStruct;
            #[automatically_derived]
            impl ::ensemble::serde::Serialize for #name {
                fn serialize<S: ::ensemble::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    let mut state = serializer.serialize_struct(stringify!(#name), #count)?;
                    #serialize_fields
                    state.end()
                }
            }
        };
    }
}

pub fn impl_deserialize(name: &Ident, fields: &Fields) -> syn::Result<TokenStream> {
    let visitor_name = Ident::new(
        &format!("__{}", format!("{name} Visitor").to_class_case()),
        name.span(),
    );
    let enum_key = &fields
        .fields
        .iter()
        .map(|f| Ident::new(&f.ident.to_string().to_class_case(), f.span()))
        .collect::<Rc<_>>();

    let column = &fields
        .fields
        .iter()
        .map(|f| {
            if f.has_relationship() {
                return f.ident.clone();
            }

            f.attr
                .column
                .as_ref()
                .map_or(f.ident.clone(), |v| Ident::new(v, f.span()))
        })
        .collect::<Rc<_>>();

    let field_deserialize = field_deserialize(column, enum_key);
    let visitor_deserialize = visitor_deserialize(name, &visitor_name, fields, column, enum_key)?;

    Ok(quote! {
        const _: () = {
            use ::ensemble::serde as _serde;
            use _serde::de::IntoDeserializer;
            use ensemble::relationships::Relationship;

            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for #name {
                fn deserialize<D: _serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    enum Field { #(#enum_key,)* Other(String) };
                    #field_deserialize

                    struct #visitor_name;
                    #visitor_deserialize

                    const FIELDS: &'static [&'static str] = &[#(stringify!(#column)),*];

                    deserializer.deserialize_struct(stringify!(#name), FIELDS, #visitor_name {})
                }
            }
        };
    })
}

fn field_deserialize(column: &Rc<[Ident]>, enum_key: &Rc<[Ident]>) -> TokenStream {
    let expecting_str = column
        .iter()
        .map(|f| format!("`{}`", f.to_string()))
        .collect::<Rc<_>>()
        .join(" or ");

    quote! {
        impl<'de> _serde::Deserialize<'de> for Field {
            fn deserialize<D: _serde::de::Deserializer<'de>>(deserializer: D) -> Result<Field, D::Error> {
                struct FieldVisitor;

                impl<'de> _serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        formatter.write_str(#expecting_str)
                    }

                    fn visit_str<E: _serde::de::Error>(self, value: &str) -> Result<Field, E> {
                        match value {
                            #(stringify!(#column) => Ok(Field::#enum_key),)*
                            _ => {
                                Ok(Field::Other(::std::string::ToString::to_string(value)))
                            },
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }
    }
}

fn visitor_deserialize(
    name: &Ident,
    visitor_name: &Ident,
    fields: &Fields,
    column: &Rc<[Ident]>,
    enum_key: &Rc<[Ident]>,
) -> syn::Result<TokenStream> {
    let primary_key = fields.primary_key()?;
    let key = &fields.fields.iter().map(|f| &f.ident).collect::<Rc<_>>();

    let required_checks = fields.fields.iter().filter_map(|f| {
        let ident = &f.ident;
        let column = f
            .attr
            .column
            .as_ref()
            .map_or(f.ident.clone(), |v| Ident::new(v, f.span()));

        if f.has_relationship() {
            None
        } else {
            Some(quote_spanned! {f.span()=> let #ident = #ident.ok_or_else(|| _serde::de::Error::missing_field(stringify!(#column)))?; })
        }

    });

    let model_keys = fields.fields.iter().map(|f| {
        let ident = &f.ident;
        let Some((relationship_type, related, relationship_key)) = &f.relationship(primary_key) else {
            return quote_spanned! {f.span()=> #ident: #ident };
        };
        let relationship_ident = Ident::new(&relationship_type.to_string(), f.span());

        let key_ident = key
            .iter()
            .find(|k| &k.to_string() == relationship_key)
            .map_or_else(|| {
                quote_spanned! {f.span()=> _serde::de::Deserialize::deserialize::<_serde::__private::de::ContentDeserializer<'_, _serde::de::value::Error>>(__collect.get(#relationship_key).unwrap().clone().into_deserializer()).unwrap() }
            }, |key| quote_spanned! {f.span()=> #key });

        let foreign_key = f.foreign_key(*relationship_type);

        quote_spanned! {f.span()=> #ident: <#relationship_ident<#name, #related>>::build(#key_ident, #ident, #foreign_key) }
    });

    let build_model = quote! {
        Ok(#name { #(#model_keys),* })
    };

    Ok(quote! {
        impl<'de> _serde::de::Visitor<'de> for #visitor_name {
            type Value = #name;

            fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                formatter.write_str(&format!("struct {}", stringify!(#name)))
            }

            fn visit_map<V: _serde::de::MapAccess<'de>>(self, mut map: V) -> Result<#name, V::Error> {
                #(let mut #key = None;)*
                let mut __collect = ::std::collections::HashMap::<String, _serde::__private::de::Content>::new();

                while let Some(key) = map.next_key()? {
                    match key {
                        #(
                            Field::#enum_key => {
                                if #key.is_some() {
                                    return Err(_serde::de::Error::duplicate_field(stringify!(#column)));
                                }
                                #key = Some(map.next_value()?);
                            },
                        )*
                        Field::Other(name) => {
                            __collect.insert(name, map.next_value()?);
                        }
                    }
                }

                #(#required_checks)*

                #build_model
            }
        }
    })
}
