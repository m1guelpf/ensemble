use deluxe::ExtractAttributes;
use quote::ToTokens;
use syn::{spanned::Spanned, Attribute, Expr, FieldsNamed, Lit};

pub struct Fields {
    ast: FieldsNamed,
    pub fields: Vec<Field>,
}

impl Fields {
    pub fn separate(&self) -> (Vec<&Field>, Vec<&Field>) {
        self.fields.iter().partition(|f| f.attr.init)
    }
}

pub struct Field {
    pub attr: Attr,
    ast: syn::Field,
    pub ty: syn::Type,
    pub ident: syn::Ident,
    pub doc: Option<String>,
}

#[derive(ExtractAttributes, Default)]
#[deluxe(attributes(builder), default)]
pub struct Attr {
    pub skip: bool,
    pub init: bool,
    pub into: bool,
    pub needs: Option<Expr>,
    #[deluxe(rename = type, append)]
    pub types: Vec<Expr>,
    pub rename: Option<String>,
}

impl Field {
    pub fn new(mut field: syn::Field) -> Self {
        let ident = field.ident.clone().unwrap();
        let attr = Attr::extract_attributes(&mut field.attrs).unwrap();

        Self {
            attr,
            ident,
            ty: field.ty.clone(),
            doc: Self::get_doc(&field.attrs),
            ast: field,
        }
    }

    fn get_doc(attrs: &[Attribute]) -> Option<String> {
        attrs
            .iter()
            .find(|attr| attr.meta.path().is_ident("doc"))
            .and_then(|attr| {
                attr.meta.require_name_value().ok().and_then(|meta| {
                    let Expr::Lit(lit) = &meta.value else {
                        return None;
                    };

                    match &lit.lit {
                        Lit::Str(s) => Some(s.value()),
                        _ => None,
                    }
                })
            })
    }

    pub fn span(&self) -> proc_macro2::Span {
        self.ast.span()
    }
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ast.to_tokens(tokens);
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
