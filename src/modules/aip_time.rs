//! Defines the `time` module, used in the Lua engine.
//!
//! ---
//!
//! ## Lua documentation
//!
//! The `aip.time` module exposes functions to retrieve, offset, format, and parse date and time information.
//!
//! ### Functions
//!
//! - `aip.time.now_utc_micro(params?: TimeOffsetParams) -> integer`
//! - `aip.time.now_local(params?: TimeOffsetParams) -> AipTimeData`
//! - `aip.time.now_utc(params?: TimeOffsetParams) -> AipTimeData`
//! - `aip.time.today_local(params?: TimeOffsetParams) -> AipTimeData`
//! - `aip.time.today_utc(params?: TimeOffsetParams) -> AipTimeData`
//! - `aip.time.offset_micro(params: TimeOffsetParams) -> integer`
//! - `aip.time.from_micro(params: TimeOffsetParams) -> AipTimeData`
//! - `aip.time.from_utc_micro(params: TimeOffsetParams) -> AipTimeData`
//! - `aip.time.from_local_micro(params: TimeOffsetParams) -> AipTimeData`
//! - `aip.time.parse(params: AipTimeParseParams) -> AipTimeData`
//!
//! ---

#![allow(non_camel_case_types)]

use crate::base;
use crate::derive::{AipOutput, AipParams};
use crate::registry::{HandlerError, HandlerResult};
use crate::{AipFromLua, AipIntoLua, AipRegistry, AipRegistryBuilder, HandlerCallContext};
use aiprog_macros::{aip_handler, register_handler};
use mlua::LuaSerdeExt;
use time::OffsetDateTime;

// region:    --- Types

/// Parameters for time offset calculations and date conversions.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema, AipParams)]
#[serde_with::skip_serializing_none]
pub struct TimeOffsetParams {
	/// Base epoch timestamp in microseconds. Defaults to current timestamp or midnight.
	pub epoch_micro: Option<i64>,

	/// Whether to evaluate/format in local timezone. Defaults to false (UTC).
	pub is_local: Option<bool>,

	/// Offset in days (supports fractional and negative numbers).
	pub day: Option<f64>,

	/// Offset in hours.
	pub hour: Option<f64>,

	/// Offset in seconds.
	pub sec: Option<f64>,

	/// Offset in milliseconds.
	pub ms: Option<f64>,

	/// Offset in microseconds.
	pub micro: Option<i64>,
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

	/// True if formatted in local timezone, false if UTC.
	pub is_local: bool,
}

/// Scalar output for epoch microsecond timestamps.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema, AipOutput)]
pub struct AipTimeMicroOutput(pub i64);

// endregion: --- Types

// region:    --- Lua Traits

impl AipFromLua for TimeOffsetParams {
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
			mlua::Value::Table(ref table) => {
				// If this table is an AipTimeData record (contains rfc3339 or day_num),
				// calendar fields (day, hour, micro) must not be treated as relative offsets.
				if table.contains_key("rfc3339")? || table.contains_key("day_num")? {
					Ok(Self {
						epoch_micro: table.get("epoch_micro")?,
						is_local: table.get("is_local")?,
						..Default::default()
					})
				} else {
					lua.from_value(val).map_err(|e| format!("{e}").into())
				}
			}
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

// endregion: --- Lua Traits

// region:    --- Module Registration

/// Build and return an [`AipRegistry`] containing all `aip.time` handlers.
#[allow(dead_code)]
pub fn init_registry() -> crate::Result<AipRegistry> {
	Ok(register(AipRegistryBuilder::default())?.build())
}

pub fn register(mut registry: AipRegistryBuilder) -> crate::Result<AipRegistryBuilder> {
	register_handler!(registry, "aip.time.now_utc_micro", aip_time_now_utc_micro_handler)?;
	register_handler!(registry, "aip.time.now_local", aip_time_now_local_handler)?;
	register_handler!(registry, "aip.time.now_utc", aip_time_now_utc_handler)?;
	register_handler!(registry, "aip.time.today_local", aip_time_today_local_handler)?;
	register_handler!(registry, "aip.time.today_utc", aip_time_today_utc_handler)?;
	register_handler!(registry, "aip.time.offset_micro", aip_time_offset_micro_handler)?;
	register_handler!(registry, "aip.time.from_micro", aip_time_from_micro_handler)?;
	register_handler!(registry, "aip.time.from_utc_micro", aip_time_from_utc_micro_handler)?;
	register_handler!(registry, "aip.time.from_local_micro", aip_time_from_local_micro_handler)?;
	register_handler!(registry, "aip.time.parse", aip_time_parse_handler)?;
	Ok(registry)
}

// endregion: --- Module Registration

// region:    --- Handlers

/// Returns current Unix epoch time in microseconds, with optional offset.
#[aip_handler]
fn aip_time_now_utc_micro_handler(
	_call: HandlerCallContext,
	params: TimeOffsetParams,
) -> HandlerResult<AipTimeMicroOutput> {
	let base = params.epoch_micro.unwrap_or_else(base::time::now_micro);
	let offset = base::time::compute_micro_offset(params.day, params.hour, params.sec, params.ms, params.micro);
	Ok(AipTimeMicroOutput(base.saturating_add(offset)))
}

/// Returns current local date-time data, with optional offset.
#[aip_handler]
fn aip_time_now_local_handler(_call: HandlerCallContext, params: TimeOffsetParams) -> HandlerResult<AipTimeData> {
	let base = params.epoch_micro.unwrap_or_else(base::time::now_micro);
	let offset = base::time::compute_micro_offset(params.day, params.hour, params.sec, params.ms, params.micro);
	let target_micro = base.saturating_add(offset);
	let dt = base::time::epoch_micro_to_local_datetime(target_micro).map_err(HandlerError::custom_from_err)?;
	build_aip_time_data(&dt, true).map_err(HandlerError::custom_from_err)
}

/// Returns current UTC date-time data, with optional offset.
#[aip_handler]
fn aip_time_now_utc_handler(_call: HandlerCallContext, params: TimeOffsetParams) -> HandlerResult<AipTimeData> {
	let base = params.epoch_micro.unwrap_or_else(base::time::now_micro);
	let offset = base::time::compute_micro_offset(params.day, params.hour, params.sec, params.ms, params.micro);
	let target_micro = base.saturating_add(offset);
	let dt = base::time::epoch_micro_to_utc_datetime(target_micro).map_err(HandlerError::custom_from_err)?;
	build_aip_time_data(&dt, false).map_err(HandlerError::custom_from_err)
}

/// Returns local date-time data anchored at today's midnight (00:00:00.000000), with optional offset.
#[aip_handler]
fn aip_time_today_local_handler(_call: HandlerCallContext, params: TimeOffsetParams) -> HandlerResult<AipTimeData> {
	let base = base::time::today_local_midnight_micro().map_err(HandlerError::custom_from_err)?;
	let offset = base::time::compute_micro_offset(params.day, params.hour, params.sec, params.ms, params.micro);
	let target_micro = base.saturating_add(offset);
	let dt = base::time::epoch_micro_to_local_datetime(target_micro).map_err(HandlerError::custom_from_err)?;
	build_aip_time_data(&dt, true).map_err(HandlerError::custom_from_err)
}

/// Returns UTC date-time data anchored at today's midnight (00:00:00.000000), with optional offset.
#[aip_handler]
fn aip_time_today_utc_handler(_call: HandlerCallContext, params: TimeOffsetParams) -> HandlerResult<AipTimeData> {
	let base = base::time::today_utc_midnight_micro().map_err(HandlerError::custom_from_err)?;
	let offset = base::time::compute_micro_offset(params.day, params.hour, params.sec, params.ms, params.micro);
	let target_micro = base.saturating_add(offset);
	let dt = base::time::epoch_micro_to_utc_datetime(target_micro).map_err(HandlerError::custom_from_err)?;
	build_aip_time_data(&dt, false).map_err(HandlerError::custom_from_err)
}

/// Applies offsets to a given epoch microsecond timestamp.
#[aip_handler]
fn aip_time_offset_micro_handler(
	_call: HandlerCallContext,
	params: TimeOffsetParams,
) -> HandlerResult<AipTimeMicroOutput> {
	let base = params.epoch_micro.unwrap_or_else(base::time::now_micro);
	let offset = base::time::compute_micro_offset(params.day, params.hour, params.sec, params.ms, params.micro);
	Ok(AipTimeMicroOutput(base.saturating_add(offset)))
}

/// Converts an epoch microsecond timestamp to date-time data, respecting is_local and optional offsets.
#[aip_handler]
fn aip_time_from_micro_handler(_call: HandlerCallContext, params: TimeOffsetParams) -> HandlerResult<AipTimeData> {
	let base = params.epoch_micro.unwrap_or_else(base::time::now_micro);
	let offset = base::time::compute_micro_offset(params.day, params.hour, params.sec, params.ms, params.micro);
	let target_micro = base.saturating_add(offset);
	let is_local = params.is_local.unwrap_or(false);
	let dt = if is_local {
		base::time::epoch_micro_to_local_datetime(target_micro).map_err(HandlerError::custom_from_err)?
	} else {
		base::time::epoch_micro_to_utc_datetime(target_micro).map_err(HandlerError::custom_from_err)?
	};
	build_aip_time_data(&dt, is_local).map_err(HandlerError::custom_from_err)
}

/// Converts an epoch microsecond timestamp to UTC date-time data.
#[aip_handler]
fn aip_time_from_utc_micro_handler(_call: HandlerCallContext, params: TimeOffsetParams) -> HandlerResult<AipTimeData> {
	let base = params.epoch_micro.unwrap_or_else(base::time::now_micro);
	let offset = base::time::compute_micro_offset(params.day, params.hour, params.sec, params.ms, params.micro);
	let target_micro = base.saturating_add(offset);
	let dt = base::time::epoch_micro_to_utc_datetime(target_micro).map_err(HandlerError::custom_from_err)?;
	build_aip_time_data(&dt, false).map_err(HandlerError::custom_from_err)
}

/// Converts an epoch microsecond timestamp to local date-time data.
#[aip_handler]
fn aip_time_from_local_micro_handler(
	_call: HandlerCallContext,
	params: TimeOffsetParams,
) -> HandlerResult<AipTimeData> {
	let base = params.epoch_micro.unwrap_or_else(base::time::now_micro);
	let offset = base::time::compute_micro_offset(params.day, params.hour, params.sec, params.ms, params.micro);
	let target_micro = base.saturating_add(offset);
	let dt = base::time::epoch_micro_to_local_datetime(target_micro).map_err(HandlerError::custom_from_err)?;
	build_aip_time_data(&dt, true).map_err(HandlerError::custom_from_err)
}

/// Parses an RFC 3339 or ISO 8601 string into an AipTimeData record.
#[aip_handler]
fn aip_time_parse_handler(_call: HandlerCallContext, params: AipTimeParseParams) -> HandlerResult<AipTimeData> {
	let dt = base::time::parse_rfc3339_or_iso8601(&params.text)
		.map_err(|e| HandlerError::custom(format!("aip.time.parse failed: {e}")))?;
	let is_local = if let Ok(local_offset) = time::UtcOffset::current_local_offset() {
		dt.offset() == local_offset
	} else {
		false
	};
	build_aip_time_data(&dt, is_local).map_err(HandlerError::custom_from_err)
}

// endregion: --- Handlers

// region:    --- Support

fn build_aip_time_data(dt: &OffsetDateTime, is_local: bool) -> crate::Result<AipTimeData> {
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
		is_local,
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
	async fn test_aip_time_now_utc_micro_simple() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			return aip.time.now_utc_micro()
		"#;
		let res = _test_support::eval_script(&engine, script).await?;
		let micro = res.as_i64().ok_or("Expected integer")?;
		assert!(micro > 1_700_000_000_000_000);
		Ok(())
	}

	#[tokio::test]
	async fn test_aip_time_now_utc_micro_with_offsets() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			local base = 1000000000
			local shifted = aip.time.now_utc_micro({
				epoch_micro = base,
				day = 1,
				hour = 2,
				sec = 30,
				ms = 500,
				micro = 100
			})
			return shifted
		"#;
		let res = _test_support::eval_script(&engine, script).await?;
		let shifted = res.as_i64().ok_or("Expected integer")?;
		let expected_offset = (86_400_000_000i64) + (2 * 3_600_000_000i64) + (30 * 1_000_000i64) + (500 * 1_000) + 100;
		assert_eq!(shifted, 1_000_000_000 + expected_offset);
		Ok(())
	}

	#[tokio::test]
	async fn test_aip_time_now_utc_and_local() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			local utc = aip.time.now_utc()
			local local_t = aip.time.now_local()
			return { utc = utc, local_t = local_t }
		"#;
		let res = _test_support::eval_script(&engine, script).await?;

		let utc = &res["utc"];
		assert_eq!(utc["is_local"], false);
		assert_eq!(utc["utc_offset_seconds"], 0);
		assert!(utc["year"].as_i64().unwrap() >= 2026);
		assert!(utc["month"].as_u64().unwrap() >= 1 && utc["month"].as_u64().unwrap() <= 12);
		assert!(utc["day_num"].as_u64().unwrap() >= 1 && utc["day_num"].as_u64().unwrap() <= 7);
		assert!(!utc["day_name"].as_str().unwrap().is_empty());
		assert!(!utc["rfc3339"].as_str().unwrap().is_empty());

		let local_t = &res["local_t"];
		assert_eq!(local_t["is_local"], true);
		assert!(local_t["year"].as_i64().unwrap() >= 2026);
		Ok(())
	}

	#[tokio::test]
	async fn test_aip_time_today_utc_and_local() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			local today_utc = aip.time.today_utc()
			local next_week_utc = aip.time.today_utc({ day = 7 })
			return { today = today_utc, next_week = next_week_utc }
		"#;
		let res = _test_support::eval_script(&engine, script).await?;

		let today = &res["today"];
		assert_eq!(today["hour"], 0);
		assert_eq!(today["minute"], 0);
		assert_eq!(today["second"], 0);
		assert_eq!(today["micro"], 0);
		assert_eq!(today["is_local"], false);

		let next_week = &res["next_week"];
		let today_micro = today["epoch_micro"].as_i64().unwrap();
		let next_week_micro = next_week["epoch_micro"].as_i64().unwrap();
		assert_eq!(next_week_micro - today_micro, 7 * 86_400_000_000);
		assert_eq!(next_week["day_name"], today["day_name"]);
		assert_eq!(next_week["day_num"], today["day_num"]);
		Ok(())
	}

	#[tokio::test]
	async fn test_aip_time_offset_micro() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			local base = 5000000000
			local shifted = aip.time.offset_micro({
				epoch_micro = base,
				day = -1.5,
				hour = 12
			})
			return shifted
		"#;
		let res = _test_support::eval_script(&engine, script).await?;
		let shifted = res.as_i64().ok_or("Expected integer")?;
		let expected_delta = (-1.5 * 86_400_000_000.0) as i64 + (12 * 3_600_000_000i64);
		assert_eq!(shifted, 5_000_000_000 + expected_delta);
		Ok(())
	}

	#[tokio::test]
	async fn test_aip_time_from_micro_roundtrip() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			local original = aip.time.now_utc()
			local roundtripped = aip.time.from_micro(original)
			local tomorrow = aip.time.from_micro({
				epoch_micro = original.epoch_micro,
				is_local = original.is_local,
				day = 1
			})
			return { orig = original, round = roundtripped, tom = tomorrow }
		"#;
		let res = _test_support::eval_script(&engine, script).await?;
		assert_eq!(res["orig"]["epoch_micro"], res["round"]["epoch_micro"]);
		assert_eq!(res["orig"]["rfc3339"], res["round"]["rfc3339"]);

		let orig_micro = res["orig"]["epoch_micro"].as_i64().unwrap();
		let tom_micro = res["tom"]["epoch_micro"].as_i64().unwrap();
		assert_eq!(tom_micro - orig_micro, 86_400_000_000);
		Ok(())
	}

	#[tokio::test]
	async fn test_aip_time_from_utc_and_local_micro() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			local base = 1787421612000000
			local as_utc = aip.time.from_utc_micro({ epoch_micro = base })
			local as_local = aip.time.from_local_micro({ epoch_micro = base })
			return { utc = as_utc, local_t = as_local }
		"#;
		let res = _test_support::eval_script(&engine, script).await?;
		assert_eq!(res["utc"]["is_local"], false);
		assert_eq!(res["utc"]["epoch_micro"], 1787421612000000i64);
		assert_eq!(res["local_t"]["is_local"], true);
		assert_eq!(res["local_t"]["epoch_micro"], 1787421612000000i64);

		// Test scalar integer input directly
		let script_scalar = r#"
			local as_utc = aip.time.from_utc_micro(1787421612000000)
			return as_utc
		"#;
		let res_scalar = _test_support::eval_script(&engine, script_scalar).await?;
		assert_eq!(res_scalar["epoch_micro"], 1787421612000000i64);
		assert_eq!(res_scalar["is_local"], false);

		Ok(())
	}

	#[tokio::test]
	async fn test_aip_time_parse() -> Result<()> {
		let engine = _test_support::setup_lua_engine(init_registry)?;
		let script = r#"
			local parsed1 = aip.time.parse({ text = "2026-08-22T15:30:45Z" })
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
