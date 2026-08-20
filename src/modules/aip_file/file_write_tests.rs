type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::*;
use mlua::Lua;

#[test]
fn test_format_content_variations() {
	let input = "  \nhello world\n\n\n".to_string();
	let formatted = format_content(input, Some(true), Some(true), Some(true));
	assert_eq!(formatted, "hello world\n");
}

#[test]
fn test_write_params_from_lua() -> Result<()> {
	let lua = Lua::new();
	let table = lua.create_table()?;
	table.set("path", "foo/bar.txt")?;
	table.set("content", "hello")?;
	table.set("trim_start", true)?;
	table.set("single_trailing_newline", true)?;

	let params = AipFileWriteParams::from_lua(&lua, Value::Table(table))?;
	assert_eq!(params.path, "foo/bar.txt");
	assert_eq!(params.content, "hello");
	assert_eq!(params.trim_start, Some(true));
	assert_eq!(params.trim_end, None);
	assert_eq!(params.single_trailing_newline, Some(true));
	Ok(())
}

#[test]
fn test_copy_move_params_from_lua() -> Result<()> {
	let lua = Lua::new();
	let table = lua.create_table()?;
	table.set("src", "a.txt")?;
	table.set("dest", "b.txt")?;
	table.set("overwrite", true)?;

	let copy_params = AipFileCopyParams::from_lua(&lua, Value::Table(table.clone()))?;
	assert_eq!(copy_params.src, "a.txt");
	assert_eq!(copy_params.dest, "b.txt");
	assert_eq!(copy_params.overwrite, Some(true));

	let move_params = AipFileMoveParams::from_lua(&lua, Value::Table(table))?;
	assert_eq!(move_params.src, "a.txt");
	assert_eq!(move_params.dest, "b.txt");
	assert_eq!(move_params.overwrite, Some(true));
	Ok(())
}

#[test]
fn test_delete_params_from_lua() -> Result<()> {
	let lua = Lua::new();
	let table = lua.create_table()?;
	table.set("path", "file.txt")?;
	table.set("base_dir", "/tmp")?;

	let params = AipFileDeleteParams::from_lua(&lua, Value::Table(table))?;
	assert_eq!(params.path, "file.txt");
	assert_eq!(params.base_dir, Some("/tmp".to_string()));
	Ok(())
}

#[test]
fn test_write_append_delete_handlers() -> Result<()> {
	let temp_dir = std::env::temp_dir().join(format!(
		"aiprog_test_write_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	std::fs::create_dir_all(&temp_dir)?;

	let dir_ctx =
		DirContext::from_base_dir(temp_dir.to_str().unwrap()).map_err(|e| crate::Error::custom(e.to_string()))?;

	let mut running_ctx = crate::running_context::RunningContext::default();
	running_ctx.insert(dir_ctx);
	let handle = crate::running_context::RunningContextHandle::new(running_ctx);
	let call = HandlerCallContext::new(handle);

	// Write
	let write_params = AipFileWriteParams {
		path: "sub/test.txt".to_string(),
		content: "  initial content\n".to_string(),
		base_dir: None,
		trim_start: Some(true),
		trim_end: None,
		single_trailing_newline: Some(true),
	};
	let write_out =
		aip_file_write_handler(call.clone(), write_params).map_err(|e| crate::Error::custom(e.to_string()))?;
	assert_eq!(write_out.0.name, "test.txt");
	assert_eq!(write_out.0.path, "sub/test.txt");

	let read_content = std::fs::read_to_string(temp_dir.join("sub/test.txt"))?;
	assert_eq!(read_content, "initial content\n");

	// Append
	let append_params = AipFileAppendParams {
		path: "sub/test.txt".to_string(),
		content: "appended line\n".to_string(),
		base_dir: None,
	};
	let append_out =
		aip_file_append_handler(call.clone(), append_params).map_err(|e| crate::Error::custom(e.to_string()))?;
	assert_eq!(append_out.0.name, "test.txt");

	let read_after_append = std::fs::read_to_string(temp_dir.join("sub/test.txt"))?;
	assert_eq!(read_after_append, "initial content\nappended line\n");

	// Delete existing
	let delete_params = AipFileDeleteParams {
		path: "sub/test.txt".to_string(),
		base_dir: None,
	};
	let delete_out =
		aip_file_delete_handler(call.clone(), delete_params).map_err(|e| crate::Error::custom(e.to_string()))?;
	assert!(delete_out.0);
	assert!(!temp_dir.join("sub/test.txt").exists());

	// Delete non-existing
	let delete_params_missing = AipFileDeleteParams {
		path: "sub/test.txt".to_string(),
		base_dir: None,
	};
	let delete_missing_out =
		aip_file_delete_handler(call, delete_params_missing).map_err(|e| crate::Error::custom(e.to_string()))?;
	assert!(!delete_missing_out.0);

	let _ = std::fs::remove_dir_all(&temp_dir);
	Ok(())
}

#[test]
fn test_copy_and_move_handlers() -> Result<()> {
	let temp_dir = std::env::temp_dir().join(format!(
		"aiprog_test_copy_move_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	std::fs::create_dir_all(&temp_dir)?;

	let dir_ctx =
		DirContext::from_base_dir(temp_dir.to_str().unwrap()).map_err(|e| crate::Error::custom(e.to_string()))?;

	let mut running_ctx = crate::running_context::RunningContext::default();
	running_ctx.insert(dir_ctx);
	let handle = crate::running_context::RunningContextHandle::new(running_ctx);
	let call = HandlerCallContext::new(handle);

	// Prepare source file
	let src_file = temp_dir.join("origin.txt");
	std::fs::write(&src_file, "original content")?;

	// Copy without overwrite
	let copy_params = AipFileCopyParams {
		src: "origin.txt".to_string(),
		dest: "nested/copy.txt".to_string(),
		base_dir: None,
		overwrite: Some(false),
	};
	let copy_out = aip_file_copy_handler(call.clone(), copy_params).map_err(|e| crate::Error::custom(e.to_string()))?;
	assert_eq!(copy_out.0.name, "copy.txt");
	assert_eq!(copy_out.0.path, "nested/copy.txt");
	assert_eq!(
		std::fs::read_to_string(temp_dir.join("nested/copy.txt"))?,
		"original content"
	);
	assert!(src_file.exists());

	// Copy conflict without overwrite fails
	let copy_conflict_params = AipFileCopyParams {
		src: "origin.txt".to_string(),
		dest: "nested/copy.txt".to_string(),
		base_dir: None,
		overwrite: Some(false),
	};
	let copy_err = aip_file_copy_handler(call.clone(), copy_conflict_params);
	assert!(copy_err.is_err());

	// Copy conflict with overwrite succeeds
	std::fs::write(&src_file, "updated content")?;
	let copy_overwrite_params = AipFileCopyParams {
		src: "origin.txt".to_string(),
		dest: "nested/copy.txt".to_string(),
		base_dir: None,
		overwrite: Some(true),
	};
	let copy_overwrite_out =
		aip_file_copy_handler(call.clone(), copy_overwrite_params).map_err(|e| crate::Error::custom(e.to_string()))?;
	assert_eq!(copy_overwrite_out.0.name, "copy.txt");
	assert_eq!(
		std::fs::read_to_string(temp_dir.join("nested/copy.txt"))?,
		"updated content"
	);

	// Move file
	let move_params = AipFileMoveParams {
		src: "origin.txt".to_string(),
		dest: "moved/target.txt".to_string(),
		base_dir: None,
		overwrite: Some(false),
	};
	let move_out = aip_file_move_handler(call.clone(), move_params).map_err(|e| crate::Error::custom(e.to_string()))?;
	assert_eq!(move_out.0.name, "target.txt");
	assert_eq!(move_out.0.path, "moved/target.txt");
	assert_eq!(
		std::fs::read_to_string(temp_dir.join("moved/target.txt"))?,
		"updated content"
	);
	assert!(!src_file.exists());

	let _ = std::fs::remove_dir_all(&temp_dir);
	Ok(())
}

#[test]
fn test_ensure_exists_and_ensure_dir_handlers() -> Result<()> {
	let temp_dir = std::env::temp_dir().join(format!(
		"aiprog_test_ensure_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	std::fs::create_dir_all(&temp_dir)?;

	let dir_ctx =
		DirContext::from_base_dir(temp_dir.to_str().unwrap()).map_err(|e| crate::Error::custom(e.to_string()))?;

	let mut running_ctx = crate::running_context::RunningContext::default();
	running_ctx.insert(dir_ctx);
	let handle = crate::running_context::RunningContextHandle::new(running_ctx);
	let call = HandlerCallContext::new(handle);

	// Ensure dir creates new directory hierarchy
	let ensure_dir_params = AipFileEnsureDirParams {
		path: "nested/a/b".to_string(),
		base_dir: None,
	};
	let created_dir = aip_file_ensure_dir_handler(call.clone(), ensure_dir_params)
		.map_err(|e| crate::Error::custom(e.to_string()))?;
	assert!(created_dir.0);
	assert!(temp_dir.join("nested/a/b").is_dir());

	// Ensure dir returns false when already present
	let ensure_dir_again_params = AipFileEnsureDirParams {
		path: "nested/a/b".to_string(),
		base_dir: None,
	};
	let created_dir_again = aip_file_ensure_dir_handler(call.clone(), ensure_dir_again_params)
		.map_err(|e| crate::Error::custom(e.to_string()))?;
	assert!(!created_dir_again.0);

	// Ensure exists creates missing file with initial content
	let ensure_file_params = AipFileEnsureExistsParams {
		path: "nested/a/b/file.txt".to_string(),
		content: Some("initial content".to_string()),
		base_dir: None,
		content_when_empty: None,
	};
	let file_out = aip_file_ensure_exists_handler(call.clone(), ensure_file_params)
		.map_err(|e| crate::Error::custom(e.to_string()))?;
	assert_eq!(file_out.0.name, "file.txt");
	assert_eq!(
		std::fs::read_to_string(temp_dir.join("nested/a/b/file.txt"))?,
		"initial content"
	);

	// Ensure exists on non-empty file does not overwrite content
	let ensure_existing_file_params = AipFileEnsureExistsParams {
		path: "nested/a/b/file.txt".to_string(),
		content: Some("overwritten content".to_string()),
		base_dir: None,
		content_when_empty: Some(true),
	};
	let file_existing_out = aip_file_ensure_exists_handler(call.clone(), ensure_existing_file_params)
		.map_err(|e| crate::Error::custom(e.to_string()))?;
	assert_eq!(file_existing_out.0.name, "file.txt");
	assert_eq!(
		std::fs::read_to_string(temp_dir.join("nested/a/b/file.txt"))?,
		"initial content"
	);

	// Ensure exists on empty file fills content when content_when_empty is true
	let empty_file_path = temp_dir.join("nested/a/b/empty.txt");
	std::fs::write(&empty_file_path, "")?;
	let ensure_empty_file_params = AipFileEnsureExistsParams {
		path: "nested/a/b/empty.txt".to_string(),
		content: Some("populated content".to_string()),
		base_dir: None,
		content_when_empty: Some(true),
	};
	let file_empty_out = aip_file_ensure_exists_handler(call, ensure_empty_file_params)
		.map_err(|e| crate::Error::custom(e.to_string()))?;
	assert_eq!(file_empty_out.0.name, "empty.txt");
	assert_eq!(std::fs::read_to_string(empty_file_path)?, "populated content");

	let _ = std::fs::remove_dir_all(&temp_dir);
	Ok(())
}

#[test]
fn test_write_init_registry() -> Result<()> {
	let registry = init_registry()?;
	let handlers = registry.list_registered_fns();
	assert!(handlers.iter().any(|f| f.path == "aip.file.write"));
	assert!(handlers.iter().any(|f| f.path == "aip.file.append"));
	assert!(handlers.iter().any(|f| f.path == "aip.file.copy"));
	assert!(handlers.iter().any(|f| f.path == "aip.file.move"));
	assert!(handlers.iter().any(|f| f.path == "aip.file.delete"));
	assert!(handlers.iter().any(|f| f.path == "aip.file.ensure_exists"));
	assert!(handlers.iter().any(|f| f.path == "aip.file.ensure_dir"));

	let merged = crate::modules::aip_file::register::init_registry()?;
	let merged_handlers = merged.list_registered_fns();
	assert!(merged_handlers.iter().any(|f| f.path == "aip.file.read"));
	assert!(merged_handlers.iter().any(|f| f.path == "aip.file.write"));
	Ok(())
}
