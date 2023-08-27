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
    let fields = fields.fields.iter().map(|field| {
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

    quote! {
        const _: () = {
            use ::ensemble::serde::ser::SerializeStruct;
            #[automatically_derived]
            impl ::ensemble::serde::Serialize for #name {
                fn serialize<S: ::ensemble::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    let mut state = serializer.serialize_struct(stringify!(#name), #count)?;
                    #(#fields)*
                    state.end()
                }
            }
        };
    }
}

pub fn impl_deserialize(name: &Ident, fields: &Fields) -> TokenStream {
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
            f.attr
                .column
                .as_ref()
                .map_or(f.ident.clone(), |v| Ident::new(v, f.span()))
        })
        .collect::<Rc<_>>();

    let field_deserialize = field_deserialize(column, enum_key);
    let visitor_deserialize = visitor_deserialize(name, &visitor_name, fields, column, enum_key);

    quote! {
        const _: () = {
            use ::ensemble::serde as _serde;

            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for #name {
                fn deserialize<D: _serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    enum Field<'de> { #(#enum_key,)* Other(_serde::__private::de::Content<'de>) };
                    #field_deserialize

                    struct #visitor_name <'de> {
                        marker: ::std::marker::PhantomData<#name>,
                        lifetime: ::std::marker::PhantomData<&'de ()>,
                    };
                    #visitor_deserialize

                    const FIELDS: &'static [&'static str] = &[#(stringify!(#column)),*];

                    deserializer.deserialize_struct(stringify!(#name), FIELDS, #visitor_name {
                        marker: ::std::marker::PhantomData::<#name>,
                        lifetime: ::std::marker::PhantomData,
                    })
                }
            }
        };
    }
}

fn field_deserialize(column: &Rc<[Ident]>, enum_key: &Rc<[Ident]>) -> TokenStream {
    let expecting_str = column
        .iter()
        .map(|f| format!("`{}`", f.to_string()))
        .collect::<Rc<_>>()
        .join(" or ");

    quote! {
        impl<'de> _serde::Deserialize<'de> for Field<'de> {
            fn deserialize<D: _serde::de::Deserializer<'de>>(deserializer: D) -> Result<Field<'de>, D::Error> {
                struct FieldVisitor;

                impl<'de> _serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field<'de>;

                    fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        formatter.write_str(#expecting_str)
                    }

                    fn visit_str<E: _serde::de::Error>(self, value: &str) -> Result<Field<'de>, E> {
                        match value {
                            #(stringify!(#column) => Ok(Field::#enum_key),)*
                            _ => {
                                let value = _serde::__private::de::Content::String(
                                    ::std::string::ToString::to_string(value),
                                );
                                Ok(Field::Other(value))
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
) -> TokenStream {
    let key = &fields.fields.iter().map(|f| &f.ident).collect::<Rc<_>>();
    let seq_iter = fields.fields.iter().enumerate().map(|(i, field)| {
        let ident = &field.ident;
        quote! { let #ident = seq.next_element()?.ok_or_else(|| _serde::de::Error::invalid_length(#i, &self))? }
    });

    let model_keys = fields.fields.iter().map(|f| {
        let ident = &f.ident;
                quote_spanned! {f.span()=> #ident: #ident}
    });

    let build_model = quote! {
        let mut model = #name { #(#model_keys),* };
        model.hydrate();

        Ok(model)
    };

    quote! {
        impl<'de> _serde::de::Visitor<'de> for #visitor_name <'de> {
            type Value = #name;

            fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                formatter.write_str(&format!("struct {}", stringify!(#name)))
            }

            fn visit_seq<V: _serde::de::SeqAccess<'de>>(self, mut seq: V) -> Result<#name, V::Error> {
                #(#seq_iter;)*

                #build_model
            }

            fn visit_map<V: _serde::de::MapAccess<'de>>(self, mut map: V) -> Result<#name, V::Error> {
                #(let mut #key = None;)*
                let mut __collect = ::std::vec::Vec::<Option<(_serde::__private::de::Content, _serde::__private::de::Content)>>::new();

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
                            __collect.push(Some((name, map.next_value()?)))
                        }
                    }
                }
                #(let #key = #key.ok_or_else(|| _serde::de::Error::missing_field(stringify!(#column)))?;)*

                #build_model
            }
        }
    }
}
