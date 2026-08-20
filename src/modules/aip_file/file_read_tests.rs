type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::super::file_types::{AbsolutePathPolicy, DirPolicyError, PathPolicy};
use super::*;
use crate::{RunningContext, ScriptEngine};
use tempfile::TempDir;

#[tokio::test]
async fn test_read_file_ok() -> Result<()> {
	let tmp = TempDir::new()?;
	let file_path = tmp.path().join("hello.txt");
	std::fs::write(&file_path, "world")?;

	// Build FileContext using SPath
	let (_root, dir_context) = setup_test_context(&tmp)?;

	// Register the single handler directly via the registry (for unit test)
	let back = eval_file_script("return aip.file.read({ path = 'hello.txt' })", dir_context).await?;

	assert_eq!(back["content"], serde_json::json!("world"));
	Ok(())
}

#[test]
fn test_aip_file_dir_context_separate_read_write_permissions() -> Result<()> {
	// -- Setup & Fixtures
	let read_tmp = TempDir::new()?;
	let write_tmp = TempDir::new()?;
	let read_root = simple_fs::SPath::from_std_path(read_tmp.path())?;
	let write_root = simple_fs::SPath::from_std_path(write_tmp.path())?;
	let read_policy = PathPolicy::new([read_root.clone()], AbsolutePathPolicy::Allow)?;
	let write_policy = PathPolicy::new([write_root.clone()], AbsolutePathPolicy::Allow)?;
	let context = DirContext::new(read_root, read_policy, write_policy)?;

	// -- Exec
	let readable = context.resolve_read_target("input.txt", None)?;
	let writable = context.resolve_write("output/new.txt", Some(write_root.as_str()))?;
	let denied_write = context.resolve_write("../outside.txt", Some(write_root.as_str()));

	// -- Check
	assert!(readable.path().as_str().contains(read_tmp.path().to_string_lossy().as_ref()));
	assert!(writable.path().as_str().contains(write_tmp.path().to_string_lossy().as_ref()));
	assert!(denied_write.is_err());
	Ok(())
}

#[test]
fn test_aip_file_dir_context_denies_absolute_paths() -> Result<()> {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let root = simple_fs::SPath::from_std_path(tmp.path())?;
	let policy = PathPolicy::new([root.clone()], AbsolutePathPolicy::Deny)?;
	let context = DirContext::new(root.clone(), policy.clone(), policy)?;

	// -- Exec
	let result = context.resolve_read(root.as_str(), None);

	// -- Check
	assert!(matches!(result, Err(DirPolicyError::AbsolutePathDenied(_))));
	Ok(())
}

#[test]
fn test_aip_file_dir_context_temporary_assertions_return_true() -> Result<()> {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let root = simple_fs::SPath::from_std_path(tmp.path())?;
	let policy = PathPolicy::new([root.clone()], AbsolutePathPolicy::Deny)?;
	let context = DirContext::new(root.clone(), policy.clone(), policy)?;

	// -- Exec
	let readable = context.assert_read(&root)?;
	let writable = context.assert_write(&root)?;

	// -- Check
	assert!(readable);
	assert!(writable);
	Ok(())
}

#[cfg(unix)]
#[test]
fn test_aip_file_dir_context_denies_symlink_escape() -> Result<()> {
	use std::os::unix::fs::symlink;

	// -- Setup & Fixtures
	let allowed = TempDir::new()?;
	let outside = TempDir::new()?;
	std::fs::write(outside.path().join("secret.txt"), "secret")?;
	symlink(outside.path(), allowed.path().join("escape"))?;
	let root = simple_fs::SPath::from_std_path(allowed.path())?;
	let policy = PathPolicy::new([root.clone()], AbsolutePathPolicy::Allow)?;
	let context = DirContext::new(root, policy.clone(), policy)?;

	// -- Exec
	let result = context.resolve_read("escape/secret.txt", None);

	// -- Check
	assert!(matches!(result, Err(DirPolicyError::OutsideAllowedRoots(_))));
	Ok(())
}

#[test]
fn test_aip_file_dir_context_default() -> Result<()> {
	let context = DirContext::default();
	let current_dir = simple_fs::SPath::from_std_path(std::env::current_dir()?)?;
	let canonical_current_dir = current_dir.canonicalize()?;

	let resolved = context.resolve_read_target("Cargo.toml", None)?;
	assert_eq!(resolved.root().as_str(), canonical_current_dir.as_str());

	let denied_absolute = context.resolve_read(current_dir.as_str(), None);
	assert!(matches!(denied_absolute, Err(DirPolicyError::AbsolutePathDenied(_))));
	Ok(())
}

#[tokio::test]
async fn test_aip_file_dir_context_subfolder_base_dir() -> Result<()> {
	let tmp = TempDir::new()?;
	let sub_dir = tmp.path().join("sub");
	std::fs::create_dir_all(&sub_dir)?;
	std::fs::write(sub_dir.join("sub_hello.txt"), "hello from sub")?;

	let sub_spath = simple_fs::SPath::from_std_path(&sub_dir)?;
	let dir_context = DirContext::from_base_dir(sub_spath)?;

	let back = eval_file_script("return aip.file.read({ path = 'sub_hello.txt' })", dir_context).await?;

	assert_eq!(back["content"], serde_json::json!("hello from sub"));
	Ok(())
}

#[tokio::test]
async fn test_aip_file_dir_context_base_dir_param_override() -> Result<()> {
	let tmp = TempDir::new()?;
	let sub_dir = tmp.path().join("sub");
	std::fs::create_dir_all(&sub_dir)?;
	std::fs::write(sub_dir.join("nested.txt"), "nested content")?;

	let (_root, dir_context) = setup_test_context(&tmp)?;

	let back = eval_file_script(
		"return aip.file.read({ path = 'nested.txt', base_dir = 'sub' })",
		dir_context,
	)
	.await?;

	assert_eq!(back["content"], serde_json::json!("nested content"));
	Ok(())
}

#[test]
fn test_aip_file_dir_context_invalid_base_dir_outside_roots() -> Result<()> {
	let tmp_allowed = TempDir::new()?;
	let tmp_outside = TempDir::new()?;

	let allowed_root = simple_fs::SPath::from_std_path(tmp_allowed.path())?;
	let outside_dir = simple_fs::SPath::from_std_path(tmp_outside.path())?;

	let policy = PathPolicy::new([allowed_root], AbsolutePathPolicy::Allow)?;
	let result = DirContext::new(outside_dir, policy.clone(), policy);

	assert!(matches!(result, Err(DirPolicyError::OutsideAllowedRoots(_))));
	Ok(())
}

// region:    --- Test Support

fn setup_test_engine() -> Result<ScriptEngine> {
	let registry = super::init_registry()?;
	let engine = ScriptEngine::builder().with_registry(registry).build()?;
	Ok(engine)
}

fn setup_test_context(tmp: &TempDir) -> Result<(simple_fs::SPath, DirContext)> {
	let root = simple_fs::SPath::from_std_path(tmp.path())?;
	let context = DirContext::from_base_dir(root.clone())?;
	Ok((root, context))
}

async fn eval_file_script(script: &str, dir_context: DirContext) -> Result<serde_json::Value> {
	let engine = setup_test_engine()?;
	let mut context = RunningContext::default();
	context.insert(dir_context);

	let outcome = engine.exec(script, context).await?;
	let value = outcome.result?;
	Ok(value)
}

// endregion: --- Test Support
