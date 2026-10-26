use crate::Error;
use crate::store::Namespace;

/// The separator used to flatten a [`Namespace`] into a single column value.
const SEP: char = '/';

/// Flattens a namespace into a length-prefixed, `/`-joined string.
///
/// The length prefix makes the encoding unambiguous even when a segment
/// contains the separator, so decoding always recovers the exact segments.
pub(crate) fn encode_namespace(namespace: &[String]) -> String {
    namespace
        .iter()
        .map(|segment| format!("{}:{segment}", segment.len()))
        .collect::<Vec<_>>()
        .join(&SEP.to_string())
}

/// Reverses [`encode_namespace`].
pub(crate) fn decode_namespace(encoded: &str) -> Result<Namespace, Error> {
    if encoded.is_empty() {
        return Ok(Vec::new());
    }
    encoded
        .split(SEP)
        .map(|part| {
            let (len, rest) = part
                .split_once(':')
                .ok_or_else(|| malformed(format!("malformed namespace segment: {part}")))?;
            let len: usize = len
                .parse()
                .map_err(|_| malformed(format!("malformed namespace length: {len}")))?;
            if rest.len() != len {
                return Err(malformed(format!(
                    "namespace segment length mismatch: {part}"
                )));
            }
            Ok(rest.to_string())
        })
        .collect()
}

/// Returns the LIKE pattern matching `prefix` and any deeper namespace.
///
/// `\_`, `\%`, and `\\` are escape sequences under both
/// `ESCAPE '\'` (SQLite) and `ESCAPE '\\'` (MySQL and PostgreSQL), so the same
/// pattern works for every backend.
pub(crate) fn prefix_like(prefix: &[String]) -> String {
    if prefix.is_empty() {
        return "%".to_string();
    }
    // Escape LIKE metacharacters in the encoded prefix, then append a wildcard
    // to match any deeper namespace.
    let encoded = encode_namespace(prefix);
    let escaped = encoded
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("{escaped}%")
}

fn malformed(message: String) -> Error {
    Error::Persistence { message }
}
