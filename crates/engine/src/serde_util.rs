//! Serde helpers shared by every persisted struct (§4.2 serde policy).

/// UUID fields: accept any case on read, emit uppercase-hyphenated on write —
/// byte-compatible with the legacy encoders.
pub mod uuid_upper {
    use serde::{Deserialize, Deserializer, Serializer};
    use uuid::Uuid;

    /// Serialize as `7B4C2A10-53E5-4D2A-9C6F-2E8B1F0A9D11`.
    pub fn serialize<S: Serializer>(id: &Uuid, serializer: S) -> Result<S::Ok, S::Error> {
        let mut buffer = Uuid::encode_buffer();
        serializer.serialize_str(id.hyphenated().encode_upper(&mut buffer))
    }

    /// Deserialize accepting any case.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Uuid, D::Error> {
        Uuid::deserialize(deserializer)
    }
}

/// `Option<Uuid>` variant of [`uuid_upper`].
pub mod uuid_upper_opt {
    use serde::{Deserialize, Deserializer, Serializer};
    use uuid::Uuid;

    /// Serialize `Some` as uppercase-hyphenated, `None` as null.
    pub fn serialize<S: Serializer>(id: &Option<Uuid>, serializer: S) -> Result<S::Ok, S::Error> {
        match id {
            Some(id) => super::uuid_upper::serialize(id, serializer),
            None => serializer.serialize_none(),
        }
    }

    /// Deserialize accepting any case or null.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Uuid>, D::Error> {
        Option::<Uuid>::deserialize(deserializer)
    }
}

/// Snapshot dates: RFC-3339 UTC with whole seconds on write
/// (`2026-07-27T18:04:11Z`); fractional seconds accepted on read (§4.3).
pub mod utc_seconds {
    use chrono::{DateTime, SecondsFormat, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    /// Serialize as `2026-07-27T18:04:11Z`.
    pub fn serialize<S: Serializer>(
        date: &DateTime<Utc>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&date.to_rfc3339_opts(SecondsFormat::Secs, true))
    }

    /// Deserialize any RFC-3339 timestamp, normalizing to UTC.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<DateTime<Utc>, D::Error> {
        let raw = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&raw)
            .map(|parsed| parsed.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }
}
