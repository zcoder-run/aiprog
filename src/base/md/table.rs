#![allow(unused)]

// region:    --- CellValue

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum CellValue {
	String(String),
	Number(f64),
	Bool(bool),
	Null,
}

impl CellValue {
	pub fn as_str(&self) -> String {
		match self {
			CellValue::String(s) => s.clone(),
			CellValue::Number(n) => {
				if n.fract() == 0.0 && !n.is_infinite() && !n.is_nan() && *n <= i64::MAX as f64 && *n >= i64::MIN as f64
				{
					format!("{}", *n as i64)
				} else {
					format!("{}", n)
				}
			}
			CellValue::Bool(b) => {
				if *b {
					"true".to_string()
				} else {
					"false".to_string()
				}
			}
			CellValue::Null => String::new(),
		}
	}
}

impl From<String> for CellValue {
	fn from(s: String) -> Self {
		CellValue::String(s)
	}
}

impl From<&str> for CellValue {
	fn from(s: &str) -> Self {
		CellValue::String(s.to_string())
	}
}

impl From<bool> for CellValue {
	fn from(b: bool) -> Self {
		CellValue::Bool(b)
	}
}

impl From<i64> for CellValue {
	fn from(n: i64) -> Self {
		CellValue::Number(n as f64)
	}
}

impl From<i32> for CellValue {
	fn from(n: i32) -> Self {
		CellValue::Number(n as f64)
	}
}

impl From<f64> for CellValue {
	fn from(n: f64) -> Self {
		CellValue::Number(n)
	}
}

// endregion: --- CellValue

// region:    --- Table Generator

/// Minimum character width for any Markdown table column delimiter (`---`).
const MIN_COL_WIDTH: usize = 3;

/// Generates a formatted Markdown pipe table string.
pub fn make_table<S: AsRef<str>>(headers: Option<&[S]>, rows: &[Vec<CellValue>]) -> String {
	let mut num_cols = 0;

	if let Some(hdrs) = headers {
		num_cols = num_cols.max(hdrs.len());
	}

	for row in rows {
		num_cols = num_cols.max(row.len());
	}

	if num_cols == 0 {
		return String::new();
	}

	let mut col_widths = vec![MIN_COL_WIDTH; num_cols];

	if let Some(hdrs) = headers {
		for (i, h) in hdrs.iter().enumerate() {
			col_widths[i] = col_widths[i].max(h.as_ref().len());
		}
	}

	for row in rows {
		for (i, cell) in row.iter().enumerate() {
			let s = cell.as_str();
			col_widths[i] = col_widths[i].max(s.len());
		}
	}

	let mut lines: Vec<String> = Vec::new();

	if let Some(hdrs) = headers {
		let header_cells: Vec<String> = (0..num_cols)
			.map(|i| {
				let content = hdrs.get(i).map(|s| s.as_ref()).unwrap_or("");
				format!(" {:<width$} ", content, width = col_widths[i])
			})
			.collect();
		lines.push(format!("|{}|", header_cells.join("|")));

		let delimiter_cells: Vec<String> = (0..num_cols).map(|i| format!(" {} ", "-".repeat(col_widths[i]))).collect();
		lines.push(format!("|{}|", delimiter_cells.join("|")));
	}

	for row in rows {
		let row_cells: Vec<String> = (0..num_cols)
			.map(|i| {
				let content = row.get(i).map(|c| c.as_str()).unwrap_or_default();
				format!(" {:<width$} ", content, width = col_widths[i])
			})
			.collect();
		lines.push(format!("|{}|", row_cells.join("|")));
	}

	lines.join("\n")
}

// endregion: --- Table Generator

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_table_basic() {
		let headers = vec!["Name", "Age", "City"];
		let rows = vec![
			vec![CellValue::from("Alice"), CellValue::from(30), CellValue::from("New York")],
			vec![CellValue::from("Bob"), CellValue::from(25), CellValue::from("San Francisco")],
		];

		let table = make_table(Some(&headers), &rows);
		let expected = "\
| Name  | Age | City          |
| ----- | --- | ------------- |
| Alice | 30  | New York      |
| Bob   | 25  | San Francisco |";

		assert_eq!(table, expected);
	}

	#[test]
	fn test_table_types_and_null() {
		let headers = vec!["Col A", "Col B", "Col C", "Col D"];
		let rows = vec![
			vec![
				CellValue::from("Text"),
				CellValue::from(42.5),
				CellValue::from(true),
				CellValue::Null,
			],
			vec![
				CellValue::from("More"),
				CellValue::from(100),
				CellValue::from(false),
				CellValue::from("Present"),
			],
		];

		let table = make_table(Some(&headers), &rows);
		let expected = "\
| Col A | Col B | Col C | Col D   |
| ----- | ----- | ----- | ------- |
| Text  | 42.5  | true  |         |
| More  | 100   | false | Present |";

		assert_eq!(table, expected);
	}

	#[test]
	fn test_table_uneven_rows_and_headers() {
		let headers = vec!["H1"];
		let rows = vec![
			vec![CellValue::from("A"), CellValue::from("B"), CellValue::from("C")],
			vec![CellValue::from("D")],
		];

		let table = make_table(Some(&headers), &rows);
		let expected = "\
| H1  |     |     |
| --- | --- | --- |
| A   | B   | C   |
| D   |     |     |";

		assert_eq!(table, expected);
	}

	#[test]
	fn test_table_no_headers() {
		let rows = vec![
			vec![CellValue::from("X"), CellValue::from("Y")],
			vec![CellValue::from("1"), CellValue::from("2")],
		];

		let table = make_table::<&str>(None, &rows);
		let expected = "\
| X   | Y   |
| 1   | 2   |";

		assert_eq!(table, expected);
	}

	#[test]
	fn test_table_empty() {
		let table = make_table::<&str>(None, &[]);
		assert_eq!(table, "");

		let headers: Vec<&str> = vec![];
		let table_empty_headers = make_table(Some(&headers), &[]);
		assert_eq!(table_empty_headers, "");
	}
}

// endregion: --- Tests
