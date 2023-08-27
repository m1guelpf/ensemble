use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{spanned::Spanned, DeriveInput, Expr, GenericArgument, PathArguments, Type};

use self::field::{Field, Fields};

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
            let option = get_option_inner(ty);
            let ty = option.unwrap_or(ty);
            let is_string = ty.to_token_stream().to_string() == "String";
            let ty = if is_string {
                quote_spanned! {f.span()=> &str }
            } else {
                quote_spanned! {f.span()=> #ty }
            };
            let fn_constrain = if f.attr.into {
                quote_spanned! {f.span()=> <T: Into<#ty>> }
            } else {
                TokenStream::new()
            };
            let fn_ty = if f.attr.into {
                quote_spanned! {f.span()=> T }
            } else {
                quote_spanned! {f.span()=> #ty }
            };
            let iden = &f.ident;
            let assign = build_assign(f, is_string, option.is_some());

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

            let needs = build_needs(f)?;

            Ok(quote_spanned! {f.span()=>
                #[allow(clippy::return_self_not_must_use, clippy::must_use_candidate)]
                pub fn #alias #fn_constrain (mut self, #iden: #fn_ty) -> Self {
                    #types_constraint
                    #needs

                    self.#iden = #assign;
                    self
                }
            })
        })
        .collect()
}

fn build_assign(field: &Field, is_string: bool, is_option: bool) -> TokenStream {
    let iden = &field.ident;

    let assign = if field.attr.into {
        quote_spanned! {field.span()=> #iden.into() }
    } else if is_string {
        quote_spanned! {field.span()=> #iden.to_string() }
    } else {
        quote_spanned! {field.span()=> #iden }
    };

    if is_option {
        quote_spanned! {field.span()=> Some(#assign) }
    } else {
        quote_spanned! {field.span()=> #assign }
    }
}

fn build_needs(field: &Field) -> syn::Result<TokenStream> {
    let Some(needs) = &field.attr.needs else {
        return Ok(TokenStream::new());
    };

    let Expr::Array(array) = needs else {
        return Err(syn::Error::new_spanned(
            needs,
            "needs must be a path expression",
        ));
    };

    let mut tokens = TokenStream::new();
    let segments = &array.elems;

    for (i, segment) in segments.iter().enumerate() {
        let ident = &segment;

        if i == 0 {
            tokens.extend(quote_spanned! {segment.span()=> !self.#ident });
        } else {
            tokens.extend(quote_spanned! {segment.span()=> && !self.#ident });
        }
    }

    let iden = &field.ident;
    Ok(quote_spanned! {field.span()=>
        if #tokens {
            panic!("{} requires one of {} to be set.", stringify!(#iden), stringify!(#needs));
        }
    })
}

fn get_option_inner(r#type: &Type) -> Option<&Type> {
    let Type::Path(path) = r#type else {
        return None;
        // path.path.segments.first().unwrap().ident == "Option"
    };

    if path.path.segments.first().unwrap().ident != "Option" {
        return None;
    }

    let PathArguments::AngleBracketed(args) = &path.path.segments.first().unwrap().arguments else {
        return None;
    };

    let GenericArgument::Type(ty) = args.args.first().unwrap() else {
        return None;
    };

    Some(ty)
}
