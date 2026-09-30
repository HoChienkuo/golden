use syn::{Ident, Type};

/// One struct field contributing a property to the derived schema.
pub struct SchemaField {
    pub ident: Ident,
    pub ty: Type,
    pub description: Option<String>,
    pub required: Option<bool>,
}
