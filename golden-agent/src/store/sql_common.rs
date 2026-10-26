use serde_json::Value;
use sqlx::FromRow;

use crate::Error;
use crate::store::StoreItem;
use crate::store::ns::decode_namespace;

/// Maps any displayable error into [`Error::Persistence`].
pub(crate) fn persistence(err: impl std::fmt::Display) -> Error {
    Error::Persistence {
        message: err.to_string(),
    }
}

/// An item row as stored in the `store_items` table.
#[derive(FromRow)]
pub(crate) struct ItemRow {
    pub namespace: String,
    pub key: String,
    pub value: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Reconstructs a [`StoreItem`] from its stored row.
pub(crate) fn row_to_item(row: ItemRow) -> Result<StoreItem, Error> {
    Ok(StoreItem {
        namespace: decode_namespace(&row.namespace)?,
        key: row.key,
        value: serde_json::from_str(&row.value).map_err(persistence)?,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// The `value` column value of an item.
pub(crate) fn value_json(value: &Value) -> Result<String, Error> {
    serde_json::to_string(value).map_err(persistence)
}

/// Re-exports the namespace codec so store backends can reach it through one
/// place.
pub(crate) use crate::store::ns::{encode_namespace as encode, prefix_like};
