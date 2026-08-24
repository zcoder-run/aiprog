//! Defines the `time` module, used in the Lua engine.
//!
//! ## Lua functions
//!
//! - `aip.time.now() -> integer`
//! - `aip.time.local_utc_offset_seconds() -> integer`
//! - `aip.time.to_time_data(params: integer | ToTimeDataParams) -> AipTimeData`
//! - `aip.time.offset_by(params: OffsetByParams) -> AipTimeData`
//! - `aip.time.parse(params: string | AipTimeParseParams) -> AipTimeData`

#![allow(non_camel_case_types)]

use crate::base;
use crate::derive::{AipOutput, AipParams};
use crate::registry::{HandlerError, HandlerResult};
use crate::{AipFromLua, AipIntoLua, AipModule, AipRegistry, AipRegistryBuilder, HandlerCallContext};
use aiprog_macros::{aip_handler, register_handler};
use mlua::LuaSerdeExt;
use time::OffsetDateTime;

// region:    --- Types

/// Empty parameters for `aip.time.now`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema, AipParams)]
pub struct AipTimeNowParams {}

/// Empty parameters for `aip.time.local_utc_offset_seconds`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema, AipParams)]
pub struct AipTimeLocalOffsetParams {}

/// Parameters for converting epoch microseconds to decomposed date-time fields.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema, AipParams)]
#[serde_with::skip_serializing_none]
pub struct ToTimeDataParams {
	/// Unix epoch timestamp in microseconds.
	pub epoch_micro: i64,

	/// Explicit UTC offset in seconds (default 0 for UTC, e.g. -18000 for EST, 19800 for IST).
	pub utc_offset_seconds: Option<i32>,
}

/// Parameters for relative time offset adjustments and decomposed date formatting.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema, AipParams)]
#[serde_with::skip_serializing_none]
pub struct OffsetByParams {
	/// Base epoch timestamp in microseconds. Defaults to current timestamp.
	pub epoch_micro: Option<i64>,

	/// UTC offset in seconds for presentation formatting (default 0 for UTC).
	pub utc_offset_seconds: Option<i32>,

	/// Offset in days (supports fractional and negative numbers).
	pub by_days: Option<f64>,

	/// Offset in hours.
	pub by_hours: Option<f64>,

	/// Offset in minutes.
	pub by_minutes: Option<f64>,

	/// Offset in seconds.
	pub by_seconds: Option<f64>,

	/// Offset in milliseconds.
	pub by_ms: Option<f64>,

	/// Offset in microseconds.
	pub by_micro: Option<i64>,
}

/// Parameters for parsing date-time text.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, AipParams)]
pub struct AipTimeParseParams {
	/// ISO 8601 or RFC 3339 formatted timestamp string.
	pub text: String,
}

/// Structured date-time output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, AipParams, AipOutput)]
pub struct AipTimeData {
	/// Unix epoch timestamp in microseconds.
	pub epoch_micro: i64,

	/// Normalized RFC 3339 / ISO 8601 formatted string.
	pub rfc3339: String,

	/// Full English weekday name (e.g. "Saturday").
	pub day_name: String,

	/// 1-based ISO weekday number (1 = Monday, 7 = Sunday).
	pub day_num: u8,

	/// Calendar year (e.g. 2026).
	pub year: i32,

	/// Month of the year (1..12).
	pub month: u8,

	/// Day of the month (1..31).
	pub day: u8,

	/// Hour of the day (0..23).
	pub hour: u8,

	/// Minute of the hour (0..59).
	pub minute: u8,

	/// Second of the minute (0..59).
	pub second: u8,

	/// Microseconds fraction (0..999999).
	pub micro: u32,

	/// UTC offset in seconds (e.g. 0 for UTC, -25200 for PDT).
	pub utc_offset_seconds: i32,
}

/// Scalar output for epoch microsecond timestamps.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema, AipOutput)]
pub struct AipTimeMicroOutput(pub i64);

/// Scalar output for UTC offset in seconds.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema, AipOutput)]
pub struct AipTimeOffsetSecondsOutput(pub i32);

// endregion: --- Types

// region:    --- Lua Traits

impl AipFromLua for AipTimeNowParams {
	fn from_lua(_lua: &mlua::Lua, _val: mlua::Value) -> crate::Result<Self> {
		Ok(Self {})
	}
}

impl AipFromLua for AipTimeLocalOffsetParams {
	fn from_lua(_lua: &mlua::Lua, _val: mlua::Value) -> crate::Result<Self> {
		Ok(Self {})
	}
}

impl AipFromLua for ToTimeDataParams {
	fn from_lua(lua: &mlua::Lua, val: mlua::Value) -> crate::Result<Self> {
		match val {
			mlua::Value::Integer(i) => Ok(Self {
				epoch_micro: i,
				utc_offset_seconds: None,
			}),
			mlua::Value::Number(n) => Ok(Self {
				epoch_micro: n as i64,
				utc_offset_seconds: None,
			}),
			_ => lua.from_value(val).map_err(|e| format!("{e}").into()),
		}
	}
}

impl AipFromLua for OffsetByParams {
	fn from_lua(lua: &mlua::Lua, val: mlua::Value) -> crate::Result<Self> {
		match val {
			mlua::Value::Nil => Ok(Self::default()),
			mlua::Value::Integer(i) => Ok(Self {
				epoch_micro: Some(i),
				..Default::default()
			}),
			mlua::Value::Number(n) => Ok(Self {
				epoch_micro: Some(n as i64),
				..Default::default()
			}),
			_ => lua.from_value(val).map_err(|e| format!("{e}").into()),
		}
	}
}

impl AipFromLua for AipTimeParseParams {
	fn from_lua(lua: &mlua::Lua, val: mlua::Value) -> crate::Result<Self> {
		match val {
			mlua::Value::String(s) => Ok(Self {
				text: s.to_str().map_err(|e| format!("{e}"))?.to_string(),
			}),
			_ => lua.from_value(val).map_err(|e| format!("{e}").into()),
		}
	}
}

impl AipFromLua for AipTimeData {
	fn from_lua(lua: &mlua::Lua, val: mlua::Value) -> crate::Result<Self> {
		lua.from_value(val).map_err(|e| format!("{e}").into())
	}
}

impl AipIntoLua for AipTimeData {
	fn into_lua(self, lua: &mlua::Lua) -> crate::Result<mlua::Value> {
		lua.to_value(&self).map_err(|e| format!("{e}").into())
	}
}

impl AipIntoLua for AipTimeMicroOutput {
	fn into_lua(self, _lua: &mlua::Lua) -> crate::Result<mlua::Value> {
		Ok(mlua::Value::Integer(self.0))
	}
}

impl AipIntoLua for AipTimeOffsetSecondsOutput {
	fn into_lua(self, _lua: &mlua::Lua) -> crate::Result<mlua::Value> {
		Ok(mlua::Value::Integer(self.0 as i64))
	}
}

// endregion: --- Lua Traits

// region:    --- Module Registration

#[derive(Debug, Clone, Copy, Default)]
pub struct TimeModule;

impl AipModule for TimeModule {
	fn register(builder: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
		register(builder)
	}
}

/// Build and return an [`AipRegistry`] containing all `aip.time` handlers.
#[allow(dead_code)]
pub fn init_registry() -> crate::Result<AipRegistry> {
	Ok(AipRegistryBuilder::default().add_module(TimeModule)?.build())
}

pub fn register(mut registry: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
	register_handler!(registry, "aip.time.now", aip_time_now_handler)?;
	register_handler!(registry, "aip.time.local_utc_offset_seconds", aip_time_local_utc_offset_seconds_handler)?;
	register_handler!(registry, "aip.time.to_time_data", aip_time_to_time_data_handler)?;
	register_handler!(registry, "aip.time.offset_by", aip_time_offset_by_handler)?;
	register_handler!(registry, "aip.time.parse", aip_time_parse_handler)?;
	Ok(registry)
}

// endregion: --- Module Registration

// region:    --- Handlers

/// Returns current Unix epoch time in microseconds as a scalar integer.
#[aip_handler]
fn aip_time_now_handler(_call: HandlerCallContext, _params: AipTimeNowParams) -> HandlerResult<AipTimeMicroOutput> {
	Ok(AipTimeMicroOutput(base::time::now_micro()))
}

/// Returns current system local timezone UTC offset in seconds as a scalar integer.
#[aip_handler]
fn aip_time_local_utc_offset_seconds_handler(
	_call: HandlerCallContext,
	_params: AipTimeLocalOffsetParams,
) -> HandlerResult<AipTimeOffsetSecondsOutput> {
	let offset_secs = base::time::current_local_offset_seconds().map_err(HandlerError::custom_from_err)?;
	Ok(AipTimeOffsetSecondsOutput(offset_secs))
}

/// Decomposes an epoch microsecond timestamp into structured calendar fields.
#[aip_handler]
fn aip_time_to_time_data_handler(_call: HandlerCallContext, params: ToTimeDataParams) -> HandlerResult<AipTimeData> {
	let offset_secs = params.utc_offset_seconds.unwrap_or(0);
	let utc_offset = time::UtcOffset::from_whole_seconds(offset_secs)
		.map_err(|e| HandlerError::custom(format!("Invalid utc_offset_seconds '{offset_secs}': {e}")))?;
	let dt = base::time::epoch_micro_to_offset_datetime(params.epoch_micro, utc_offset)
		.map_err(HandlerError::custom_from_err)?;
	build_aip_time_data(&dt).map_err(HandlerError::custom_from_err)
}

/// Applies relative duration offsets to an epoch microsecond timestamp and returns decomposed date fields.
#[aip_handler]
fn aip_time_offset_by_handler(_call: HandlerCallContext, params: OffsetByParams) -> HandlerResult<AipTimeData> {
	let base_micro = params.epoch_micro.unwrap_or_else(base::time::now_micro);
	let offset = base::time::compute_relative_micro_offset(
		params.by_days,
		params.by_hours,
		params.by_minutes,
		params.by_seconds,
		params.by_ms,
		params.by_micro,
	);
	let target_micro = base_micro.saturating_add(offset);
	let offset_secs = params.utc_offset_seconds.unwrap_or(0);
	let utc_offset = time::UtcOffset::from_whole_seconds(offset_secs)
		.map_err(|e| HandlerError::custom(format!("Invalid utc_offset_seconds '{offset_secs}': {e}")))?;
	let dt = base::time::epoch_micro_to_offset_datetime(target_micro, utc_offset)
		.map_err(HandlerError::custom_from_err)?;
	build_aip_time_data(&dt).map_err(HandlerError::custom_from_err)
}

/// Parses an RFC 3339 or ISO 8601 string into an AipTimeData table.
#[aip_handler]
fn aip_time_parse_handler(_call: HandlerCallContext, params: AipTimeParseParams) -> HandlerResult<AipTimeData> {
	let dt = base::time::parse_rfc3339_or_iso8601(&params.text)
		.map_err(|e| HandlerError::custom(format!("aip.time.parse failed: {e}")))?;
	build_aip_time_data(&dt).map_err(HandlerError::custom_from_err)
}

// endregion: --- Handlers

// region:    --- Support

fn build_aip_time_data(dt: &OffsetDateTime) -> crate::Result<AipTimeData> {
	let epoch_micro = base::time::datetime_to_epoch_micro(dt);
	let rfc3339 = base::time::format_rfc3339(dt)?;
	let day_name = base::time::weekday_name(dt.weekday()).to_string();
	let day_num = base::time::weekday_number(dt.weekday());

	Ok(AipTimeData {
		epoch_micro,
		rfc3339,
		day_name,
		day_num,
		year: dt.year(),
		month: dt.month() as u8,
		day: dt.day(),
		hour: dt.hour(),
		minute: dt.minute(),
		second: dt.second(),
		micro: dt.microsecond(),
		utc_offset_seconds: dt.offset().whole_seconds(),
	})
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::_test_support;

	#[tokio::test]
	async fn test_aip_time_now_simple() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			return aip.time.now()
		"#;
		let res = _test_support::eval_script(&engine, script).await?;
		let micro = res.as_i64().ok_or("Expected integer")?;
		assert!(micro > 1_700_000_000_000_000);
		Ok(())
	}

	#[tokio::test]
	async fn test_aip_time_local_utc_offset_seconds() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			return aip.time.local_utc_offset_seconds()
		"#;
		let res = _test_support::eval_script(&engine, script).await?;
		let offset = res.as_i64().ok_or("Expected integer")?;
		assert!((-43200..=50400).contains(&offset));
		Ok(())
	}

	#[tokio::test]
	async fn test_aip_time_to_time_data() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			local base = 1787421612000000
			local utc_data = aip.time.to_time_data(base)
			local offset_data = aip.time.to_time_data({ epoch_micro = base, utc_offset_seconds = -18000 })
			return { utc = utc_data, offset = offset_data }
		"#;
		let res = _test_support::eval_script(&engine, script).await?;

		let utc = &res["utc"];
		assert_eq!(utc["epoch_micro"], 1787421612000000i64);
		assert_eq!(utc["utc_offset_seconds"], 0);
		assert_eq!(utc["year"], 2026);
		assert_eq!(utc["month"], 8);
		assert_eq!(utc["day"], 22);

		let offset = &res["offset"];
		assert_eq!(offset["epoch_micro"], 1787421612000000i64);
		assert_eq!(offset["utc_offset_seconds"], -18000);
		Ok(())
	}

	#[tokio::test]
	async fn test_aip_time_offset_by() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			local base = 1000000000
			local shifted = aip.time.offset_by({
				epoch_micro = base,
				by_days = 1,
				by_hours = 2,
				by_minutes = 15,
				by_seconds = 30,
				by_ms = 500,
				by_micro = 100
			})
			return shifted
		"#;
		let res = _test_support::eval_script(&engine, script).await?;
		let shifted_micro = res["epoch_micro"].as_i64().ok_or("Expected integer")?;
		let expected_offset = (86_400_000_000i64) + (2 * 3_600_000_000i64) + (15 * 60_000_000i64) + (30 * 1_000_000i64) + (500 * 1_000) + 100;
		assert_eq!(shifted_micro, 1_000_000_000 + expected_offset);
		Ok(())
	}

	#[tokio::test]
	async fn test_aip_time_parse() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			local parsed1 = aip.time.parse("2026-08-22T15:30:45Z")
			local parsed2 = aip.time.parse({ text = "2026-08-22T10:30:45-05:00" })
			local parsed3 = aip.time.parse({ text = "2026-08-22" })
			return { p1 = parsed1, p2 = parsed2, p3 = parsed3 }
		"#;
		let res = _test_support::eval_script(&engine, script).await?;

		let p1 = &res["p1"];
		assert_eq!(p1["year"], 2026);
		assert_eq!(p1["month"], 8);
		assert_eq!(p1["day"], 22);
		assert_eq!(p1["hour"], 15);
		assert_eq!(p1["minute"], 30);
		assert_eq!(p1["second"], 45);
		assert_eq!(p1["day_name"], "Saturday");
		assert_eq!(p1["day_num"], 6);
		assert_eq!(p1["utc_offset_seconds"], 0);

		let p2 = &res["p2"];
		assert_eq!(p2["epoch_micro"], p1["epoch_micro"]);
		assert_eq!(p2["utc_offset_seconds"], -18000);

		let p3 = &res["p3"];
		assert_eq!(p3["year"], 2026);
		assert_eq!(p3["month"], 8);
		assert_eq!(p3["day"], 22);
		assert_eq!(p3["hour"], 0);
		assert_eq!(p3["minute"], 0);
		assert_eq!(p3["second"], 0);

		Ok(())
	}

	#[tokio::test]
	async fn test_aip_time_parse_invalid() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			local ok, err = pcall(aip.time.parse, { text = "invalid-date-string" })
			if ok then
				return "should have failed"
			else
				return tostring(err)
			end
		"#;
		let res = _test_support::eval_script(&engine, script).await?;
		let err_str = res.as_str().ok_or("Expected error string")?;
		assert!(err_str.contains("aip.time.parse failed"));
		Ok(())
	}
}

// endregion: --- Tests
