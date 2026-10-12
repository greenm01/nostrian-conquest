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
            _ => Err("invalid account identity"),
        }
    }

    pub fn storage_key(&self) -> String {
        match self {
            Self::Discord(id) => format!("discord:{id}"),
            Self::Nostr(id) => format!("nostr:{id}"),
        }
    }

    pub fn from_storage_key(key: &str) -> Result<Self, &'static str> {
        let account = match key.split_once(':') {
            Some(("discord", id)) => Self::Discord(id.into()),
            Some(("nostr", id)) => Self::Nostr(id.into()),
            _ => return Err("invalid account provider"),
        };
        account.validate()?;
        Ok(account)
    }
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
