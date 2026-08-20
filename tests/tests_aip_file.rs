use aiprog::{AbsolutePathPolicy, DirContext, PathPolicy};
use aiprog::{AipRegistry, RunningContext, ScriptEngine};
use serde_json::json;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn test_aiprog_file_default_context_reads_current_dir() -> TestResult {
	// -- Setup & Fixtures
	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let lua_code = r#"
        local exists = aip.file.exists({ path = "Cargo.toml" })
        local read = aip.file.read({ path = "Cargo.toml" })
        return {
            exists = exists,
            has_content = #read.content > 0,
            name = read.name
        }
    "#;

	// -- Exec
	let result = engine.exec(lua_code, RunningContext::default()).await?.result?;

	// -- Check
	assert_eq!(result["exists"], json!(true));
	assert_eq!(result["has_content"], json!(true));
	assert_eq!(result["name"], json!("Cargo.toml"));

	Ok(())
}

#[tokio::test]
async fn test_aiprog_file_custom_dir_context_isolation() -> TestResult {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let file_path = tmp.path().join("test_file.txt");
	std::fs::write(&file_path, "custom context content")?;

	let workspace = simple_fs::SPath::from_std_path(tmp.path())?;
	let read_policy = PathPolicy::new([workspace.clone()], AbsolutePathPolicy::Deny)?;
	let write_policy = PathPolicy::new([workspace.clone()], AbsolutePathPolicy::Deny)?;
	let dir_context = DirContext::new(workspace, read_policy, write_policy)?;

	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let mut context = RunningContext::default();
	context.insert(dir_context);

	let lua_code = r#"
        local exists = aip.file.exists({ path = "test_file.txt" })
        local read = aip.file.read({ path = "test_file.txt" })
        return {
            exists = exists,
            content = read.content
        }
    "#;

	// -- Exec
	let result = engine.exec(lua_code, context).await?.result?;

	// -- Check
	assert_eq!(result["exists"], json!(true));
	assert_eq!(result["content"], json!("custom context content"));

	Ok(())
}

#[tokio::test]
async fn test_aiprog_file_custom_dir_context_denies_outside_access() -> TestResult {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let workspace = simple_fs::SPath::from_std_path(tmp.path())?;
	let read_policy = PathPolicy::new([workspace.clone()], AbsolutePathPolicy::Deny)?;
	let write_policy = PathPolicy::new([workspace.clone()], AbsolutePathPolicy::Deny)?;
	let dir_context = DirContext::new(workspace, read_policy, write_policy)?;

	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let mut context = RunningContext::default();
	context.insert(dir_context);

	let lua_code = r#"
        local read = aip.file.read({ path = "../Cargo.toml" })
        return read
    "#;

	// -- Exec
	let outcome = engine.exec(lua_code, context).await?;

	// -- Check
	assert!(outcome.result.is_err());

	Ok(())
}

#[tokio::test]
async fn test_aiprog_file_subfolder_base_dir_script_execution() -> TestResult {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let sub_path = tmp.path().join("sub");
	std::fs::create_dir_all(&sub_path)?;
	std::fs::write(sub_path.join("data.txt"), "subfolder payload")?;

	let sub_workspace = simple_fs::SPath::from_std_path(&sub_path)?;
	let dir_context = DirContext::from_base_dir(sub_workspace)?;

	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let mut context = RunningContext::default();
	context.insert(dir_context);

	let lua_code = r#"
        local exists = aip.file.exists({ path = "data.txt" })
        local read = aip.file.read({ path = "data.txt" })
        local list = aip.file.list({ globs = "*.txt" })
        return {
            exists = exists,
            content = read.content,
            list_count = #list
        }
    "#;

	// -- Exec
	let result = engine.exec(lua_code, context).await?.result?;

	// -- Check
	assert_eq!(result["exists"], json!(true));
	assert_eq!(result["content"], json!("subfolder payload"));
	assert_eq!(result["list_count"], json!(1));

	Ok(())
}

#[tokio::test]
async fn test_aiprog_file_param_base_dir_and_denial() -> TestResult {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let sub_path = tmp.path().join("nested");
	std::fs::create_dir_all(&sub_path)?;
	std::fs::write(sub_path.join("item.txt"), "item content")?;

	let workspace = simple_fs::SPath::from_std_path(tmp.path())?;
	let dir_context = DirContext::from_base_dir(workspace)?;

	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let mut context = RunningContext::default();
	context.insert(dir_context.clone());

	let ok_code = r#"
        local read = aip.file.read({ path = "item.txt", base_dir = "nested" })
        return read.content
    "#;

	let result = engine.exec(ok_code, context).await?.result?;
	assert_eq!(result, json!("item content"));

	let denied_code = r#"
        local read = aip.file.read({ path = "item.txt", base_dir = "../outside" })
        return read
    "#;

	let mut context = RunningContext::default();
	context.insert(dir_context);
	let outcome = engine.exec(denied_code, context).await?;
	assert!(outcome.result.is_err());

	Ok(())
}
