use crate::Result;
use genai::chat::Usage;
use num_format::ToFormattedString;
use simple_fs::PrettySizeOptions;
use std::time::Duration;
use time::{OffsetDateTime, format_description};

// region:    --- Spaces

pub fn spaces_up_to_10(n: u32) -> &'static str {
	match n {
		0 => "",
		1 => " ",
		2 => "  ",
		3 => "   ",
		4 => "    ",
		5 => "     ",
		6 => "      ",
		7 => "       ",
		8 => "        ",
		9 => "         ",
		10 => "          ",
		_ => "          ",
	}
}

// endregion: --- Spaces

// region:    --- Numbers

pub fn format_num(num: i64) -> String {
	num.to_formatted_string(&num_format::Locale::en)
}

pub fn float_max_dec(val: f64, max_dec: u16) -> f64 {
	let factor = 10f64.powi(max_dec as i32);
	(val * factor).round() / factor
}

pub fn format_f64(val: f64) -> String {
	let rounded = float_max_dec(val, 4);
	format!("{:.*}", 4, rounded)
}

/// Format a floating point value as a whole-number percentage string (no `%` unit).
///
/// Examples:
/// - `0.123` -> `"12"`
/// - `1.0`   -> `"100"`
pub fn format_percentage(val: f64) -> String {
	((val * 100.0).round() as i64).to_string()
}

/// Pad a number with 0 for a given max_number length
/// e.g.
/// ```rust
/// let fmt = num_pad_for_len(12, 145)
/// // fmt = "012"
/// ```
pub fn num_pad_for_len(idx: i64, max_num: usize) -> String {
	let width = if max_num == 0 {
		1
	} else {
		(max_num as f64).log10().floor() as usize + 1
	};
	format!("{:0width$}", idx + 1, width = width)
}

// endregion: --- Numbers

// region:    --- Duration

pub fn format_duration(duration: Duration) -> String {
	let duration_ms = duration.as_millis().min(u64::MAX as u128) as u64;
	let duration = if duration_ms > 10000 {
		Duration::from_secs(duration.as_secs())
	} else {
		Duration::from_millis(duration_ms)
	};
	humantime::format_duration(duration).to_string()
}

pub fn format_duration_us(duration_us: i64) -> String {
	let duration = Duration::from_micros(duration_us as u64);
	format_duration(duration)
}

// already in
pub fn format_time_local(epoch_us: i64) -> Result<String> {
	fn inner(epoch_us: i64) -> std::result::Result<String, Box<dyn std::error::Error>> {
		let secs = epoch_us / 1_000_000;
		let utc_dt = OffsetDateTime::from_unix_timestamp(secs)?;
		let local_offset = OffsetDateTime::now_local()?.offset();

		let local_dt = utc_dt.to_offset(local_offset);
		// let format = format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")?;
		let format = format_description::parse("At [hour]:[minute]:[second]")?;
		Ok(local_dt.format(&format)?)
	}

	let res = inner(epoch_us).map_err(|err| format!("Cannot format epoch_us '{epoch_us}'.\nCause: {err}"))?;

	Ok(res)
}

// endregion: --- Duration

/// Formats 9 fix chars
pub fn format_pretty_size(size_in_bytes: u64, options: Option<PrettySizeOptions>) -> String {
	let options = options.unwrap_or_default();
	simple_fs::pretty_size_with_options(size_in_bytes, options)
}

// region:    --- Genai

/// Format the `Prompt Tokens: 2,070 | Completion Tokens: 131`
pub fn format_usage(usage: &Usage) -> String {
	let mut buff = String::new();

	buff.push_str("Prompt Tokens: ");
	buff.push_str(&format_num(usage.prompt_tokens.unwrap_or_default() as i64));
	if let Some(prompt_tokens_details) = usage.prompt_tokens_details.as_ref() {
		buff.push_str(" (cached: ");
		let cached = prompt_tokens_details.cached_tokens.unwrap_or(0);
		buff.push_str(&format_num(cached as i64));
		if let Some(cache_creation_tokens) = prompt_tokens_details.cache_creation_tokens {
			buff.push_str(", cache_creation: ");
			buff.push_str(&format_num(cache_creation_tokens as i64));
		}
		buff.push(')');
	}

	buff.push_str(" | Completion Tokens: ");
	buff.push_str(&format_num(usage.completion_tokens.unwrap_or_default() as i64));
	if let Some(reasoning) = usage.completion_tokens_details.as_ref().and_then(|v| v.reasoning_tokens) {
		buff.push_str(" (reasoning: ");
		buff.push_str(&format_num(reasoning as i64));
		buff.push(')');
	}

	buff
}

// endregion: --- Genai

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;

	#[test]
	fn test_support_text_format_percentage() -> Result<()> {
		// -- Setup & Fixtures
		let cases = [
			//
			(0.0, "0"),
			(0.123, "12"),
			(0.129, "13"),
			(0.5, "50"),
			(1.0, "100"),
		];

		// -- Exec & Check
		for &(input, expected) in &cases {
			let actual = format_percentage(input);
			assert_eq!(actual, expected, "input: {input}");
		}

		Ok(())
	}
}

// endregion: --- Tests
