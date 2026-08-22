use syn::{Expr, Ident, LitStr, Type};

pub enum FieldSource {
    PathVariable {
        name: Option<LitStr>,
    },

    RequestParam {
        name: Option<LitStr>,
        default: Option<DefaultValue>,
    },

    RequestHeader {
        name: Expr,
    },

    RequestBody,
}

pub enum DefaultValue {
    DefaultTrait,
    Expression(Expr),
}

pub struct RequestField {
    pub ident: Ident,
    pub ty: Type,
    pub inner_ty: Type,
    pub optional: bool,
    pub source: FieldSource,
}

pub struct RequestOptions {
    pub rejection: Type,
    pub validate: bool,
}
