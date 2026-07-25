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
	let workspace =
		simple_fs::SPath::from_std_path(tmp.path()).map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
	let read_policy = PathPolicy::new([workspace.clone()], AbsolutePathPolicy::Deny)?;
	let write_policy = PathPolicy::new([workspace], AbsolutePathPolicy::Deny)?;
	let dir_context = DirContext::new(read_policy, write_policy);

	// Register the single handler directly via the registry (for unit test)
	let registry = super::init_registry()?;
	let template = ScriptEngine::builder().with_registry(registry).build()?;
	let mut context = RunningContext::default();
	context.insert(dir_context);

	let outcome = template.exec("return aip.file.read({ path = 'hello.txt' })", context).await?;
	let back = outcome.result?;

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
	let read_policy = PathPolicy::new([read_root], AbsolutePathPolicy::Deny)?;
	let write_policy = PathPolicy::new([write_root], AbsolutePathPolicy::Deny)?;
	let context = DirContext::new(read_policy, write_policy);

	// -- Exec
	let readable = context.resolve_read_target("input.txt", None)?;
	let writable = context.resolve_write("output/new.txt", None)?;
	let denied_write = context.resolve_write("../outside.txt", None);

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
	let context = DirContext::new(policy.clone(), policy);

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
	let context = DirContext::new(policy.clone(), policy);

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
	let policy = PathPolicy::new([root], AbsolutePathPolicy::Allow)?;
	let context = DirContext::new(policy.clone(), policy);

	// -- Exec
	let result = context.resolve_read("escape/secret.txt", None);

	// -- Check
	assert!(matches!(result, Err(DirPolicyError::OutsideAllowedRoots(_))));
	Ok(())
}
