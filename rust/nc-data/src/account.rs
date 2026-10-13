//! Verified community identities. Names are labels, never account identifiers.
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, Value, ValueRef};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "provider",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AccountIdentity {
    Discord(String),
    Nostr(String),
    /// Fully qualified Matrix user ID, such as `@alice:matrix.org`.
    Matrix(String),
}

impl AccountIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::Discord(id)
                if !id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit()) =>
            {
                Ok(())
            }
            Self::Nostr(id)
                if id.len() == 64
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
            {
                Ok(())
            }
            Self::Matrix(id) if valid_matrix_user_id(id) => Ok(()),
            _ => Err("invalid account identity"),
        }
    }

    pub fn storage_key(&self) -> String {
        match self {
            Self::Discord(id) => format!("discord:{id}"),
            Self::Nostr(id) => format!("nostr:{id}"),
            Self::Matrix(id) => format!("matrix:{id}"),
        }
    }

    pub fn from_storage_key(key: &str) -> Result<Self, &'static str> {
        let account = match key.split_once(':') {
            Some(("discord", id)) => Self::Discord(id.into()),
            Some(("nostr", id)) => Self::Nostr(id.into()),
            Some(("matrix", id)) => Self::Matrix(id.into()),
            _ => return Err("invalid account provider"),
        };
        account.validate()?;
        Ok(account)
    }
}

/// Matrix user ID grammar, accepting historical localparts: `@`, a localpart of
/// printable ASCII other than `:`, then a server name; 255 bytes at most.
fn valid_matrix_user_id(id: &str) -> bool {
    let Some((local, server)) = id.strip_prefix('@').and_then(|rest| rest.split_once(':')) else {
        return false;
    };
    id.len() <= 255
        && !local.is_empty()
        && local.bytes().all(|b| b.is_ascii_graphic())
        && !server.is_empty()
        && server
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-:[]".contains(&b))
}

impl ToSql for AccountIdentity {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.validate()
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
        Ok(ToSqlOutput::Owned(Value::Text(self.storage_key())))
    }
}

impl FromSql for AccountIdentity {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Self::from_storage_key(value.as_str()?).map_err(|e| FromSqlError::Other(e.into()))
    }
}
