use std::mem;

use jiff::{
    civil::{self, Date, DateTime, Time},
    SignedDuration, Span, Timestamp,
};

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::types::{PgInterval, Type};
use crate::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueFormat, PgValueRef, Postgres};

const POSTGRES_EPOCH_DATE: Date = civil::date(2000, 1, 1);
const POSTGRES_EPOCH_DATETIME: DateTime = civil::date(2000, 1, 1).at(0, 0, 0, 0);
const POSTGRES_EPOCH_TIMESTAMP: Timestamp = Timestamp::constant(946_684_800, 0);
const MIDNIGHT: Time = Time::midnight();

impl Type<Postgres> for Timestamp {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::TIMESTAMPTZ
    }
}

impl PgHasArrayType for Timestamp {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::TIMESTAMPTZ_ARRAY
    }
}

impl Encode<'_, Postgres> for Timestamp {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let micros: i64 = self
            .duration_since(POSTGRES_EPOCH_TIMESTAMP)
            .as_micros()
            .try_into()?;
        Encode::<Postgres>::encode(micros, buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<i64>()
    }
}

impl<'r> Decode<'r, Postgres> for Timestamp {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            PgValueFormat::Binary => {
                let micros: i64 = Decode::<Postgres>::decode(value)?;
                POSTGRES_EPOCH_TIMESTAMP.checked_add(SignedDuration::from_micros(micros))?
            }
            PgValueFormat::Text => value.as_str()?.parse()?,
        })
    }
}

impl Type<Postgres> for DateTime {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::TIMESTAMP
    }
}

impl PgHasArrayType for DateTime {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::TIMESTAMP_ARRAY
    }
}

impl Encode<'_, Postgres> for DateTime {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let micros: i64 = self
            .duration_since(POSTGRES_EPOCH_DATETIME)
            .as_micros()
            .try_into()?;
        Encode::<Postgres>::encode(micros, buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<i64>()
    }
}

impl<'r> Decode<'r, Postgres> for DateTime {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            PgValueFormat::Binary => {
                let micros: i64 = Decode::<Postgres>::decode(value)?;
                POSTGRES_EPOCH_DATETIME.checked_add(SignedDuration::from_micros(micros))?
            }
            PgValueFormat::Text => value.as_str()?.parse()?,
        })
    }
}

impl Type<Postgres> for Date {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::DATE
    }
}

impl PgHasArrayType for Date {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::DATE_ARRAY
    }
}

impl Encode<'_, Postgres> for Date {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        Encode::<Postgres>::encode((*self - POSTGRES_EPOCH_DATE).get_days(), buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<i32>()
    }
}

impl<'r> Decode<'r, Postgres> for Date {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            PgValueFormat::Binary => {
                let days: i32 = Decode::<Postgres>::decode(value)?;
                POSTGRES_EPOCH_DATE.checked_add(Span::new().try_days(days)?)?
            }
            PgValueFormat::Text => value.as_str()?.parse()?,
        })
    }
}

impl Type<Postgres> for Time {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::TIME
    }
}

impl PgHasArrayType for Time {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::TIME_ARRAY
    }
}

impl Encode<'_, Postgres> for Time {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let micros: i64 = self.duration_since(MIDNIGHT).as_micros().try_into()?;
        Encode::<Postgres>::encode(micros, buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<i64>()
    }
}

impl<'r> Decode<'r, Postgres> for Time {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            PgValueFormat::Binary => {
                let micros: i64 = Decode::<Postgres>::decode(value)?;
                MIDNIGHT.checked_add(SignedDuration::from_micros(micros))?
            }
            PgValueFormat::Text => value.as_str()?.parse()?,
        })
    }
}

impl Type<Postgres> for SignedDuration {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::INTERVAL
    }
}

impl PgHasArrayType for SignedDuration {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::INTERVAL_ARRAY
    }
}

impl Encode<'_, Postgres> for SignedDuration {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        if self.as_nanos() % 1_000 != 0 {
            return Err("PostgreSQL `INTERVAL` does not support nanosecond precision".into());
        }

        PgInterval {
            months: 0,
            days: 0,
            microseconds: self.as_micros().try_into()?,
        }
        .encode_by_ref(buf)
    }

    fn size_hint(&self) -> usize {
        2 * mem::size_of::<i64>()
    }
}

impl<'r> Decode<'r, Postgres> for SignedDuration {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let interval = PgInterval::decode(value)?;

        if interval.months != 0 {
            return Err(
                "cannot decode a PostgreSQL `INTERVAL` with months as `jiff::SignedDuration`"
                    .into(),
            );
        }

        SignedDuration::from_hours(i64::from(interval.days) * 24)
            .checked_add(SignedDuration::from_micros(interval.microseconds))
            .ok_or_else(|| {
                "PostgreSQL `INTERVAL` is out of range for `jiff::SignedDuration`".into()
            })
    }
}

impl Type<Postgres> for Span {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::INTERVAL
    }
}

impl PgHasArrayType for Span {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::INTERVAL_ARRAY
    }
}

impl<'r> Decode<'r, Postgres> for Span {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let interval = PgInterval::decode(value)?;

        let mut sign = 0;
        for component in [
            i64::from(interval.months),
            i64::from(interval.days),
            interval.microseconds,
        ] {
            if component != 0 {
                if sign != 0 && sign != component.signum() {
                    return Err(
                        "cannot decode a PostgreSQL `INTERVAL` with mixed-sign components as `jiff::Span`"
                            .into(),
                    );
                }

                sign = component.signum();
            }
        }

        Ok(Span::new()
            .try_months(interval.months)?
            .try_days(interval.days)?
            .try_microseconds(interval.microseconds)?)
    }
}
