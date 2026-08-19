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
	let write_policy = PathPolicy::new([workspace], AbsolutePathPolicy::Deny)?;
	let dir_context = DirContext::new(read_policy, write_policy);

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
	let write_policy = PathPolicy::new([workspace], AbsolutePathPolicy::Deny)?;
	let dir_context = DirContext::new(read_policy, write_policy);

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
