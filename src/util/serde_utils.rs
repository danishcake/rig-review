use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Deserializer, Serializer};

/// Deserializes a non-blank string
pub fn deserialize_non_blank_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    if s.trim().is_empty() {
        return Err(serde::de::Error::custom("string must not be blank"));
    }

    Ok(s)
}

/// Serialize a SystemTime as a posix time
pub fn serialize_system_time_as_posix_seconds<S>(
    time: &SystemTime,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let secs = time
        .duration_since(UNIX_EPOCH)
        .map_err(serde::ser::Error::custom)?
        .as_secs();

    serializer.serialize_u64(secs)
}

/// Serialize an Option<SystemTime> as a posix time
pub fn serialize_optional_system_time_as_posix_seconds<S>(
    time: &Option<SystemTime>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match time {
        Some(time) => {
            let secs = time
                .duration_since(UNIX_EPOCH)
                .map_err(serde::ser::Error::custom)?
                .as_secs();

            serializer.serialize_u64(secs)
        }
        None => serializer.serialize_none(),
    }
}
