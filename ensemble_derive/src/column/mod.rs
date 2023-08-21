use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, DeriveInput, Expr};

use self::field::Fields;

mod field;

pub fn r#impl(ast: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let syn::Data::Struct(r#struct) = &ast.data else {
        return Err(syn::Error::new_spanned(
            ast,
            "Column derive only supports structs",
        ));
    };

    let syn::Fields::Named(struct_fields) = &r#struct.fields else {
        return Err(syn::Error::new_spanned(
            ast,
            "Column derive only supports named fields",
        ));
    };

    let fields = Fields::from(struct_fields.clone());

    let new_impl = impl_new(&fields);
    let set_impls = impl_set(&fields)?;

    let name = &ast.ident;
    let gen = quote! {
        impl #name {
            #new_impl
            #set_impls
        }
    };

    Ok(gen)
}

fn impl_new(fields: &Fields) -> TokenStream {
    let (init, _) = fields.separate();

    let init_types = init.iter().map(|f| {
        let ty = &f.ty;
        let iden = &f.ident;

        quote_spanned!(f.span()=> #iden: #ty)
    });

    let construct = fields.fields.iter().map(|f| {
        let iden = &f.ident;

        if f.attr.init {
            quote_spanned!(f.span()=> #iden)
        } else {
            quote_spanned!(f.span()=> #iden: Default::default())
        }
    });

    quote! {
        pub fn new(#(#init_types),*) -> Self {
            Self {
                #(#construct),*
            }
        }
    }
}

fn impl_set(fields: &Fields) -> syn::Result<TokenStream> {
    let (_, not_init) = fields.separate();

    not_init
        .iter()
        .map(|f| {
            let ty = &f.ty;
            let iden = &f.ident;
            let only_types = &f.attr.types;
            let alias = f
                .attr
                .rename
                .as_ref()
                .map_or(iden.clone(), |s| Ident::new(s, iden.span()));

            let types_constraint = if only_types.is_empty() {
                TokenStream::new()
            } else {
                quote_spanned! {
                    f.span()=> if !matches!(self.r#type, #(#only_types)|*) {
                        panic!("{} is not a valid option for {} columns.", stringify!(#iden), self.r#type);
                    }
                }
            };

            let needs = if let Some(needs) = &f.attr.needs {
                let Expr::Array(array) = needs else {
                    return Err(syn::Error::new_spanned(
                        needs,
                        "needs must be a path expression",
                    ))
                };

                let segments = &array.elems;
                let mut tokens = TokenStream::new();

                for (i, segment) in segments.iter().enumerate() {
                    let ident = &segment;

                    if i == 0 {
                        tokens.extend(quote_spanned! {segment.span()=> !self.#ident });
                    } else {
                        tokens.extend(quote_spanned! {segment.span()=> && !self.#ident });
                    }
                }

                quote_spanned! {f.span()=>
                    if #tokens {
                        panic!("{} requires one of {} to be set.", stringify!(#iden), stringify!(#needs));
                    }
                }
            } else {
                TokenStream::new()
            };

            Ok(quote_spanned! {f.span()=>
                #[allow(clippy::return_self_not_must_use, clippy::must_use_candidate)]
                pub fn #alias(mut self, #iden: #ty) -> Self {
                    #types_constraint
                    #needs

                    self.#iden = #iden;
                    self
                }
            })
        })
        .collect()
}
