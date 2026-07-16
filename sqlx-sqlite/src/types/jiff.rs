use jiff::{
    civil::{Date, DateTime, Time},
    SignedDuration, Timestamp,
};

use crate::arguments::SqliteArgumentsBuffer;
use crate::value::ValueRef;
use crate::{
    decode::Decode,
    encode::{Encode, IsNull},
    error::BoxDynError,
    type_info::DataType,
    types::Type,
    Sqlite, SqliteTypeInfo, SqliteValueRef,
};

impl Type<Sqlite> for Timestamp {
    fn type_info() -> SqliteTypeInfo {
        SqliteTypeInfo(DataType::Datetime)
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        matches!(
            ty.0,
            DataType::Datetime
                | DataType::Text
                | DataType::Integer
                | DataType::Int4
                | DataType::Float
        )
    }
}

impl Encode<'_, Sqlite> for Timestamp {
    fn encode_by_ref(&self, buf: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode(self.to_string(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for Timestamp {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.type_info().0 {
            DataType::Text => {
                let text = value.text_borrowed()?;

                if let Ok(timestamp) = text.parse() {
                    return Ok(timestamp);
                }

                Ok(jiff::tz::TimeZone::UTC.to_timestamp(text.parse::<DateTime>()?)?)
            }
            DataType::Int4 | DataType::Integer => Ok(Timestamp::from_second(value.int64()?)?),
            DataType::Float => {
                let seconds = (value.double()? - 2_440_587.5) * 86_400.0;
                Ok(Timestamp::from_duration(
                    SignedDuration::try_from_secs_f64(seconds)?,
                )?)
            }
            _ => Err("invalid SQLite storage class for `jiff::Timestamp`".into()),
        }
    }
}

impl Type<Sqlite> for DateTime {
    fn type_info() -> SqliteTypeInfo {
        SqliteTypeInfo(DataType::Datetime)
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        <Timestamp as Type<Sqlite>>::compatible(ty)
    }
}

impl Encode<'_, Sqlite> for DateTime {
    fn encode_by_ref(&self, buf: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode(self.to_string(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for DateTime {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.type_info().0 {
            DataType::Text => {
                let text = value.text_borrowed()?;
                Ok(text.strip_suffix('Z').unwrap_or(text).parse()?)
            }
            DataType::Int4 | DataType::Integer => {
                Ok(jiff::tz::TimeZone::UTC.to_datetime(Timestamp::from_second(value.int64()?)?))
            }
            DataType::Float => {
                let seconds = (value.double()? - 2_440_587.5) * 86_400.0;
                Ok(
                    jiff::tz::TimeZone::UTC.to_datetime(Timestamp::from_duration(
                        SignedDuration::try_from_secs_f64(seconds)?,
                    )?),
                )
            }
            _ => Err("invalid SQLite storage class for `jiff::civil::DateTime`".into()),
        }
    }
}

impl Type<Sqlite> for Date {
    fn type_info() -> SqliteTypeInfo {
        SqliteTypeInfo(DataType::Date)
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        matches!(ty.0, DataType::Date | DataType::Text)
    }
}

impl Encode<'_, Sqlite> for Date {
    fn encode_by_ref(&self, buf: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode(self.to_string(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for Date {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(value.text_borrowed()?.parse()?)
    }
}

impl Type<Sqlite> for Time {
    fn type_info() -> SqliteTypeInfo {
        SqliteTypeInfo(DataType::Time)
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        matches!(ty.0, DataType::Time | DataType::Text)
    }
}

impl Encode<'_, Sqlite> for Time {
    fn encode_by_ref(&self, buf: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode(self.to_string(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for Time {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let text = value.text_borrowed()?;
        Ok(text.strip_suffix('Z').unwrap_or(text).parse()?)
    }
}
