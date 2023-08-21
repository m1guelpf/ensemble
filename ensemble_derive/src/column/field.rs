use deluxe::ExtractAttributes;
use quote::ToTokens;
use syn::{spanned::Spanned, Expr, FieldsNamed};

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
    pub ty: syn::Type,
    pub ident: syn::Ident,
    ast: syn::Field,
}

#[derive(ExtractAttributes, Default)]
#[deluxe(attributes(builder), default)]
pub struct Attr {
    pub init: bool,
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
            ast: field,
        }
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
