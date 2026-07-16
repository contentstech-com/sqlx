use byteorder::{ByteOrder, LittleEndian};
use bytes::Buf;
use jiff::{
    civil::{Date, DateTime, Time},
    SignedDuration, Timestamp,
};
use sqlx_core::database::Database;

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::{BoxDynError, UnexpectedNullError};
use crate::protocol::text::ColumnType;
use crate::type_info::MySqlTypeInfo;
use crate::types::{MySqlTime, MySqlTimeSign, Type};
use crate::{MySql, MySqlValueFormat, MySqlValueRef};

impl Type<MySql> for Timestamp {
    fn type_info() -> MySqlTypeInfo {
        MySqlTypeInfo::binary(ColumnType::Timestamp)
    }

    fn compatible(ty: &MySqlTypeInfo) -> bool {
        matches!(ty.r#type, ColumnType::Datetime | ColumnType::Timestamp)
    }
}

impl Encode<'_, MySql> for Timestamp {
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> Result<IsNull, BoxDynError> {
        Encode::<MySql>::encode(jiff::tz::TimeZone::UTC.to_datetime(*self), buf)
    }
}

impl<'r> Decode<'r, MySql> for Timestamp {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(jiff::tz::TimeZone::UTC.to_timestamp(Decode::<MySql>::decode(value)?)?)
    }
}

impl Type<MySql> for Time {
    fn type_info() -> MySqlTypeInfo {
        MySqlTime::type_info()
    }
}

impl Encode<'_, MySql> for Time {
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> Result<IsNull, BoxDynError> {
        MySqlTime::new(
            MySqlTimeSign::Positive,
            self.hour().try_into()?,
            self.minute().try_into()?,
            self.second().try_into()?,
            (self.subsec_nanosecond() / 1_000).try_into()?,
        )?
        .encode_by_ref(buf)
    }
}

impl<'r> Decode<'r, MySql> for Time {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        let time = MySqlTime::decode(value)?;

        if !time.is_valid_time_of_day() {
            return Err(
                format!("MySqlTime value out of range for `jiff::civil::Time`: {time}").into(),
            );
        }

        Ok(Time::new(
            time.hours().try_into()?,
            time.minutes().try_into()?,
            time.seconds().try_into()?,
            (time.microseconds() * 1_000).try_into()?,
        )?)
    }
}

impl From<MySqlTime> for SignedDuration {
    fn from(time: MySqlTime) -> Self {
        let sign = if time.is_negative() { -1 } else { 1 };
        SignedDuration::new(
            time.whole_seconds_signed(),
            i32::try_from(sign * i64::from(time.subsec_nanos()))
                .expect("MySQL TIME microseconds always fit in i32 nanoseconds"),
        )
    }
}

impl TryFrom<SignedDuration> for MySqlTime {
    type Error = BoxDynError;

    fn try_from(value: SignedDuration) -> Result<Self, Self::Error> {
        let sign = if value.is_negative() {
            MySqlTimeSign::Negative
        } else {
            MySqlTimeSign::Positive
        };

        Ok(MySqlTime::try_from(value.unsigned_abs())?.with_sign(sign))
    }
}

impl Type<MySql> for SignedDuration {
    fn type_info() -> MySqlTypeInfo {
        MySqlTime::type_info()
    }
}

impl Encode<'_, MySql> for SignedDuration {
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> Result<IsNull, BoxDynError> {
        MySqlTime::try_from(*self)?.encode_by_ref(buf)
    }
}

impl<'r> Decode<'r, MySql> for SignedDuration {
    fn decode(value: <MySql as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(MySqlTime::decode(value)?.into())
    }
}

impl Type<MySql> for Date {
    fn type_info() -> MySqlTypeInfo {
        MySqlTypeInfo::binary(ColumnType::Date)
    }
}

impl Encode<'_, MySql> for Date {
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> Result<IsNull, BoxDynError> {
        let year = u16::try_from(self.year())
            .map_err(|_| format!("Date out of range for MySQL: {self}"))?;

        buf.push(4);
        buf.extend_from_slice(&year.to_le_bytes());
        buf.push(self.month().try_into()?);
        buf.push(self.day().try_into()?);

        Ok(IsNull::No)
    }

    fn size_hint(&self) -> usize {
        5
    }
}

impl<'r> Decode<'r, MySql> for Date {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.format() {
            MySqlValueFormat::Binary => {
                let buf = value.as_bytes()?;

                if buf.len() < 5 {
                    return Err(UnexpectedNullError.into());
                }

                Ok(Date::new(
                    LittleEndian::read_u16(&buf[1..]).try_into()?,
                    buf[3].try_into()?,
                    buf[4].try_into()?,
                )?)
            }
            MySqlValueFormat::Text => Ok(value.as_str()?.parse()?),
        }
    }
}

impl Type<MySql> for DateTime {
    fn type_info() -> MySqlTypeInfo {
        MySqlTypeInfo::binary(ColumnType::Datetime)
    }
}

impl Encode<'_, MySql> for DateTime {
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> Result<IsNull, BoxDynError> {
        let year = u16::try_from(self.year())
            .map_err(|_| format!("DateTime out of range for MySQL: {self}"))?;
        let len = if self.hour() == 0
            && self.minute() == 0
            && self.second() == 0
            && self.subsec_nanosecond() == 0
        {
            4
        } else if self.subsec_nanosecond() == 0 {
            7
        } else {
            11
        };

        buf.push(len);
        buf.extend_from_slice(&year.to_le_bytes());
        buf.push(self.month().try_into()?);
        buf.push(self.day().try_into()?);

        if len > 4 {
            buf.push(self.hour().try_into()?);
            buf.push(self.minute().try_into()?);
            buf.push(self.second().try_into()?);
        }

        if len > 7 {
            buf.extend_from_slice(&(self.subsec_nanosecond() / 1_000).to_le_bytes());
        }

        Ok(IsNull::No)
    }

    fn size_hint(&self) -> usize {
        12
    }
}

impl<'r> Decode<'r, MySql> for DateTime {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.format() {
            MySqlValueFormat::Binary => {
                let mut buf = value.as_bytes()?;

                if buf.is_empty() {
                    return Err("empty buffer".into());
                }

                let len = buf.get_u8();

                if !matches!(len, 4 | 7 | 11) {
                    return Err(format!(
                        "expected 4, 7, or 11 bytes for MySQL DATETIME value, got {len}"
                    )
                    .into());
                }

                if buf.len() != usize::from(len) {
                    return Err(format!(
                        "expected {len} bytes for MySQL DATETIME value, got {}",
                        buf.len()
                    )
                    .into());
                }

                let nanosecond = if len > 7 {
                    let microseconds = LittleEndian::read_u32(&buf[7..]);

                    if microseconds > 999_999 {
                        return Err(format!(
                            "server returned microseconds out of range: {microseconds}"
                        )
                        .into());
                    }

                    i32::try_from(microseconds * 1_000)?
                } else {
                    0
                };

                Ok(DateTime::new(
                    LittleEndian::read_u16(buf).try_into()?,
                    buf[2].try_into()?,
                    buf[3].try_into()?,
                    if len > 4 { buf[4].try_into()? } else { 0 },
                    if len > 4 { buf[5].try_into()? } else { 0 },
                    if len > 4 { buf[6].try_into()? } else { 0 },
                    nanosecond,
                )?)
            }
            MySqlValueFormat::Text => Ok(value.as_str()?.parse()?),
        }
    }
}
