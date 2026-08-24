use crate::Result;
use std::num::NonZeroU8;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::format_description::well_known::iso8601::{Config, EncodedConfig, TimePrecision};
use time::format_description::well_known::{Iso8601, Rfc3339};
use time::{OffsetDateTime, UtcOffset};

pub const RFC3339_MICRO_CONFIG: EncodedConfig = Config::DEFAULT
	.set_time_precision(TimePrecision::Second {
		decimal_digits: NonZeroU8::new(6),
	})
	.encode();

pub const RFC3339_MICRO: Iso8601<RFC3339_MICRO_CONFIG> = Iso8601;

/// Returns the Unix Time in microseconds.
///
/// Note 1: If there is any error with `duration_since UNIX_EPOCH` (which should almost never happen),
///         it returns the start of the EPOCH.
/// Note 2: The maximum UTC epoch date that can be stored in i64 with microseconds precision
///         would be approximately `292277-01-09 ... UTC`.
///         Thus, for all practical purposes, it is sufficiently distant to be of no concern.
pub fn now_micro() -> i64 {
	let now = SystemTime::now();
	let since_the_epoch = now.duration_since(UNIX_EPOCH).unwrap_or(Duration::new(0, 0));
	since_the_epoch.as_micros().min(i64::MAX as u128) as i64
}

// region:    --- Conversions & Calculations

/// Computes total microsecond offset from relative duration fields: days, hours, minutes, seconds, milliseconds, and microseconds.
#[allow(unused)]
pub fn compute_relative_micro_offset(
	by_days: Option<f64>,
	by_hours: Option<f64>,
	by_minutes: Option<f64>,
	by_seconds: Option<f64>,
	by_ms: Option<f64>,
	by_micro: Option<i64>,
) -> i64 {
	let mut total: i64 = by_micro.unwrap_or(0);
	if let Some(d) = by_days {
		total = total.saturating_add((d * 86_400_000_000.0).round() as i64);
	}
	if let Some(h) = by_hours {
		total = total.saturating_add((h * 3_600_000_000.0).round() as i64);
	}
	if let Some(m) = by_minutes {
		total = total.saturating_add((m * 60_000_000.0).round() as i64);
	}
	if let Some(s) = by_seconds {
		total = total.saturating_add((s * 1_000_000.0).round() as i64);
	}
	if let Some(ms) = by_ms {
		total = total.saturating_add((ms * 1_000.0).round() as i64);
	}
	total
}

/// Returns the current system local UTC offset in seconds.
#[allow(unused)]
pub fn current_local_offset_seconds() -> Result<i32> {
	let local_offset = UtcOffset::current_local_offset()
		.map_err(|err| format!("Cannot get local offset, cause: {err}"))?;
	Ok(local_offset.whole_seconds())
}

/// Computes total microsecond offset from optional day, hour, sec, ms, and micro parameters.
#[allow(unused)]
pub fn compute_micro_offset(
	day: Option<f64>,
	hour: Option<f64>,
	sec: Option<f64>,
	ms: Option<f64>,
	micro: Option<i64>,
) -> i64 {
	let mut total: i64 = micro.unwrap_or(0);
	if let Some(d) = day {
		total = total.saturating_add((d * 86_400_000_000.0).round() as i64);
	}
	if let Some(h) = hour {
		total = total.saturating_add((h * 3_600_000_000.0).round() as i64);
	}
	if let Some(s) = sec {
		total = total.saturating_add((s * 1_000_000.0).round() as i64);
	}
	if let Some(m) = ms {
		total = total.saturating_add((m * 1_000.0).round() as i64);
	}
	total
}

/// Converts an epoch microsecond timestamp to a UTC `OffsetDateTime`.
pub fn epoch_micro_to_utc_datetime(epoch_micro: i64) -> Result<OffsetDateTime> {
	let nanos = (epoch_micro as i128) * 1_000;
	OffsetDateTime::from_unix_timestamp_nanos(nanos)
		.map_err(|err| format!("Invalid epoch microseconds: {epoch_micro}, cause: {err}").into())
}

/// Converts an epoch microsecond timestamp to a local `OffsetDateTime`.
#[allow(unused)]
pub fn epoch_micro_to_local_datetime(epoch_micro: i64) -> Result<OffsetDateTime> {
	let utc_dt = epoch_micro_to_utc_datetime(epoch_micro)?;
	let local_offset = UtcOffset::current_local_offset()
		.map_err(|err| format!("Cannot get local offset for {utc_dt}, cause: {err}"))?;
	Ok(utc_dt.to_offset(local_offset))
}

/// Converts an epoch microsecond timestamp to an `OffsetDateTime` with a specified `UtcOffset`.
pub fn epoch_micro_to_offset_datetime(epoch_micro: i64, offset: UtcOffset) -> Result<OffsetDateTime> {
	let utc_dt = epoch_micro_to_utc_datetime(epoch_micro)?;
	Ok(utc_dt.to_offset(offset))
}

/// Converts an `OffsetDateTime` to Unix epoch microseconds.
pub fn datetime_to_epoch_micro(dt: &OffsetDateTime) -> i64 {
	(dt.unix_timestamp_nanos() / 1_000) as i64
}

/// Formats an `OffsetDateTime` into an RFC 3339 / ISO 8601 string with 6-digit microsecond precision.
pub fn format_rfc3339(dt: &OffsetDateTime) -> Result<String> {
	dt.format(&RFC3339_MICRO)
		.map_err(|e| format!("Cannot format RFC 3339 timestamp: {e}").into())
}

/// Returns the Unix epoch timestamp in microseconds for UTC midnight (00:00:00.000000) of today.
#[allow(unused)]
pub fn today_utc_midnight_micro() -> Result<i64> {
	let now_utc = OffsetDateTime::now_utc();
	let midnight = now_utc.replace_time(time::Time::MIDNIGHT);
	Ok(datetime_to_epoch_micro(&midnight))
}

/// Returns the Unix epoch timestamp in microseconds for local midnight (00:00:00.000000) of today.
#[allow(unused)]
pub fn today_local_midnight_micro() -> Result<i64> {
	let now_utc = OffsetDateTime::now_utc();
	let local_offset = UtcOffset::current_local_offset()
		.map_err(|err| format!("Cannot get local offset for {now_utc}, cause: {err}"))?;
	let now_local = now_utc.to_offset(local_offset);
	let midnight_local = now_local.replace_time(time::Time::MIDNIGHT);
	Ok(datetime_to_epoch_micro(&midnight_local))
}

/// Parses an RFC 3339 or ISO 8601 string into an `OffsetDateTime`.
pub fn parse_rfc3339_or_iso8601(text: &str) -> Result<OffsetDateTime> {
	let trimmed = text.trim();
	if let Ok(dt) = OffsetDateTime::parse(trimmed, &Rfc3339) {
		return Ok(dt);
	}
	if let Ok(dt) = OffsetDateTime::parse(trimmed, &Iso8601::DEFAULT) {
		return Ok(dt);
	}
	if let Ok(dt) = OffsetDateTime::parse(trimmed, &Iso8601::PARSING) {
		return Ok(dt);
	}
	if let Ok(items) = time::format_description::parse_borrowed::<1>("[year]-[month]-[day] [hour]:[minute]:[second]")
		&& let Ok(pdt) = time::PrimitiveDateTime::parse(trimmed, &items)
	{
		return Ok(pdt.assume_utc());
	}
	if let Ok(items) = time::format_description::parse_borrowed::<1>("[year]-[month]-[day]")
		&& let Ok(d) = time::Date::parse(trimmed, &items)
	{
		return Ok(d.midnight().assume_utc());
	}

	Err(format!("Cannot parse date/time string '{text}' as RFC 3339 or ISO 8601").into())
}

/// Returns the English weekday name for a `time::Weekday`.
pub fn weekday_name(weekday: time::Weekday) -> &'static str {
	match weekday {
		time::Weekday::Monday => "Monday",
		time::Weekday::Tuesday => "Tuesday",
		time::Weekday::Wednesday => "Wednesday",
		time::Weekday::Thursday => "Thursday",
		time::Weekday::Friday => "Friday",
		time::Weekday::Saturday => "Saturday",
		time::Weekday::Sunday => "Sunday",
	}
}

/// Returns the 1-based ISO weekday number (1 = Monday, 7 = Sunday).
pub fn weekday_number(weekday: time::Weekday) -> u8 {
	weekday.number_from_monday()
}

// endregion: --- Conversions & Calculations

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_compute_relative_micro_offset() {
		let offset = compute_relative_micro_offset(
			Some(1.5),
			Some(2.0),
			Some(15.0),
			Some(30.0),
			Some(500.0),
			Some(123),
		);
		let expected = (1.5 * 86_400_000_000.0) as i64
			+ (2.0 * 3_600_000_000.0) as i64
			+ (15.0 * 60_000_000.0) as i64
			+ (30.0 * 1_000_000.0) as i64
			+ (500.0 * 1_000.0) as i64
			+ 123;
		assert_eq!(offset, expected);
	}

	#[test]
	fn test_current_local_offset_seconds() {
		let res = current_local_offset_seconds();
		assert!(res.is_ok());
	}

	#[test]
	fn test_compute_micro_offset() {
		let offset = compute_micro_offset(Some(1.5), Some(2.0), Some(30.0), Some(500.0), Some(123));
		let expected = (1.5 * 86_400_000_000.0) as i64
			+ (2.0 * 3_600_000_000.0) as i64
			+ (30.0 * 1_000_000.0) as i64
			+ (500.0 * 1_000.0) as i64
			+ 123;
		assert_eq!(offset, expected);
	}

	#[test]
	fn test_epoch_micro_roundtrip() -> Result<()> {
		let micro = 1_787_421_612_123_456i64;
		let dt = epoch_micro_to_utc_datetime(micro)?;
		let roundtrip = datetime_to_epoch_micro(&dt);
		assert_eq!(micro, roundtrip);
		Ok(())
	}

	#[test]
	fn test_epoch_micro_to_offset_datetime() -> Result<()> {
		let micro = 1_787_421_612_123_456i64;
		let offset = UtcOffset::from_whole_seconds(-18_000).map_err(|e| format!("{e}"))?;
		let dt = epoch_micro_to_offset_datetime(micro, offset)?;
		assert_eq!(dt.offset().whole_seconds(), -18_000);
		assert_eq!(datetime_to_epoch_micro(&dt), micro);
		Ok(())
	}

	#[test]
	fn test_format_rfc3339() -> Result<()> {
		let micro = 1_787_421_612_123_456i64;
		let dt = epoch_micro_to_utc_datetime(micro)?;
		let formatted = format_rfc3339(&dt)?;
		assert_eq!(formatted, "2026-08-22T18:00:12.123456Z");
		Ok(())
	}

	#[test]
	fn test_parse_rfc3339_or_iso8601() -> Result<()> {
		let dt = parse_rfc3339_or_iso8601("2026-08-22T10:35:56.123456-07:00")?;
		assert_eq!(dt.year(), 2026);
		assert_eq!(dt.month() as u8, 8);
		assert_eq!(dt.day(), 22);
		assert_eq!(dt.hour(), 10);
		assert_eq!(dt.minute(), 35);
		assert_eq!(dt.second(), 56);
		assert_eq!(dt.microsecond(), 123456);
		assert_eq!(dt.offset().whole_seconds(), -25200);

		let dt_utc = parse_rfc3339_or_iso8601("2026-08-22T17:35:56Z")?;
		assert_eq!(dt_utc.offset().whole_seconds(), 0);
		assert_eq!(dt_utc.hour(), 17);

		let dt_date = parse_rfc3339_or_iso8601("2026-08-22")?;
		assert_eq!(dt_date.year(), 2026);
		assert_eq!(dt_date.month() as u8, 8);
		assert_eq!(dt_date.day(), 22);
		assert_eq!(dt_date.hour(), 0);

		Ok(())
	}

	#[test]
	fn test_weekday_helpers() {
		assert_eq!(weekday_name(time::Weekday::Monday), "Monday");
		assert_eq!(weekday_number(time::Weekday::Monday), 1);
		assert_eq!(weekday_name(time::Weekday::Saturday), "Saturday");
		assert_eq!(weekday_number(time::Weekday::Saturday), 6);
		assert_eq!(weekday_name(time::Weekday::Sunday), "Sunday");
		assert_eq!(weekday_number(time::Weekday::Sunday), 7);
	}
}

// endregion: --- Tests
