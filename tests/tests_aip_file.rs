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
async fn test_aiprog_file_list_contains_literal() -> TestResult {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let workspace = simple_fs::SPath::from_std_path(tmp.path())?;
	std::fs::write(tmp.path().join("a.txt"), "Hello World\nLine 2")?;
	std::fs::write(tmp.path().join("b.txt"), "hello world\nLine 2")?;
	std::fs::write(tmp.path().join("c.txt"), "other content\nLine 2")?;

	let dir_context = DirContext::from_base_dir(workspace)?;
	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let mut context = RunningContext::default();
	context.insert(dir_context);

	let lua_code = r#"
        local match_case = aip.file.list({ globs = "*.txt", contains = "Hello World" })
        local match_nocase = aip.file.list({
            globs = "*.txt",
            contains = { text = "hello world", ignore_case = true }
        })
        local match_explicit_case = aip.file.list({
            globs = "*.txt",
            contains = { text = "Hello World", ignore_case = false }
        })

        return {
            count_case = #match_case,
            name_case = match_case[1].name,
            count_nocase = #match_nocase,
            count_explicit_case = #match_explicit_case
        }
    "#;

	// -- Exec
	let result = engine.exec(lua_code, context).await?.result?;

	// -- Check
	assert_eq!(result["count_case"], json!(1));
	assert_eq!(result["name_case"], json!("a.txt"));
	assert_eq!(result["count_nocase"], json!(2));
	assert_eq!(result["count_explicit_case"], json!(1));

	Ok(())
}

#[tokio::test]
async fn test_aiprog_file_list_contains_regex() -> TestResult {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let workspace = simple_fs::SPath::from_std_path(tmp.path())?;
	std::fs::write(tmp.path().join("item1.txt"), "pub fn test_alpha() {}")?;
	std::fs::write(tmp.path().join("item2.txt"), "pub fn TEST_beta() {}")?;
	std::fs::write(tmp.path().join("item3.txt"), "pub fn helper() {}")?;

	let dir_context = DirContext::from_base_dir(workspace)?;
	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let mut context = RunningContext::default();
	context.insert(dir_context);

	let lua_code = r#"
        local match_regex = aip.file.list({
            globs = "*.txt",
            contains = { regex = "fn\\s+test_[a-z]+" }
        })
        local match_regex_nocase = aip.file.list({
            globs = "*.txt",
            contains = { regex = "fn\\s+test_[a-z]+", ignore_case = true }
        })
        local match_inline_flag = aip.file.list({
            globs = "*.txt",
            contains = { regex = "(?i)fn\\s+test_[a-z]+", ignore_case = false }
        })
        local match_inline_disable = aip.file.list({
            globs = "*.txt",
            contains = { regex = "(?-i)TEST_beta", ignore_case = true }
        })

        return {
            count_regex = #match_regex,
            count_regex_nocase = #match_regex_nocase,
            count_inline_flag = #match_inline_flag,
            count_inline_disable = #match_inline_disable
        }
    "#;

	// -- Exec
	let result = engine.exec(lua_code, context).await?.result?;

	// -- Check
	assert_eq!(result["count_regex"], json!(1));
	assert_eq!(result["count_regex_nocase"], json!(2));
	assert_eq!(result["count_inline_flag"], json!(2));
	assert_eq!(result["count_inline_disable"], json!(1));

	Ok(())
}

#[tokio::test]
async fn test_aiprog_file_first_and_list_read_contains() -> TestResult {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let workspace = simple_fs::SPath::from_std_path(tmp.path())?;
	std::fs::write(tmp.path().join("f1.txt"), "target alpha content\n")?;
	std::fs::write(tmp.path().join("f2.txt"), "target beta content\n")?;
	std::fs::write(tmp.path().join("f3.txt"), "other content\n")?;

	let dir_context = DirContext::from_base_dir(workspace)?;
	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let mut context = RunningContext::default();
	context.insert(dir_context);

	let lua_code = r#"
        local first_match = aip.file.first({
            globs = "*.txt",
            contains = "target"
        })

        local list_read_match = aip.file.list_read({
            globs = "*.txt",
            contains = "beta"
        })

        return {
            has_first = first_match ~= nil,
            list_read_count = #list_read_match,
            list_read_name = list_read_match[1].name,
            list_read_content = list_read_match[1].content
        }
    "#;

	// -- Exec
	let result = engine.exec(lua_code, context).await?.result?;

	// -- Check
	assert_eq!(result["has_first"], json!(true));
	assert_eq!(result["list_read_count"], json!(1));
	assert_eq!(result["list_read_name"], json!("f2.txt"));
	assert_eq!(result["list_read_content"], json!("target beta content\n"));

	Ok(())
}

#[tokio::test]
async fn test_aiprog_file_stats_contains() -> TestResult {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let workspace = simple_fs::SPath::from_std_path(tmp.path())?;
	let content1 = "keyword match 12345";
	let content2 = "keyword match 67890";
	let content3 = "unrelated";
	std::fs::write(tmp.path().join("s1.txt"), content1)?;
	std::fs::write(tmp.path().join("s2.txt"), content2)?;
	std::fs::write(tmp.path().join("s3.txt"), content3)?;

	let dir_context = DirContext::from_base_dir(workspace)?;
	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let mut context = RunningContext::default();
	context.insert(dir_context);

	let lua_code = r#"
        local stats = aip.file.stats({
            globs = "*.txt",
            contains = "keyword match"
        })

        return {
            number_of_files = stats.number_of_files,
            total_size = stats.total_size,
            has_ctime = stats.ctime_first ~= nil,
            has_mtime = stats.mtime_first ~= nil
        }
    "#;

	// -- Exec
	let result = engine.exec(lua_code, context).await?.result?;

	// -- Check
	assert_eq!(result["number_of_files"], json!(2));
	assert_eq!(result["total_size"], json!(content1.len() + content2.len()));
	assert_eq!(result["has_ctime"], json!(true));
	assert_eq!(result["has_mtime"], json!(true));

	Ok(())
}

#[tokio::test]
async fn test_aiprog_file_contains_invalid_regex() -> TestResult {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let workspace = simple_fs::SPath::from_std_path(tmp.path())?;
	std::fs::write(tmp.path().join("a.txt"), "hello")?;

	let dir_context = DirContext::from_base_dir(workspace)?;
	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let mut context = RunningContext::default();
	context.insert(dir_context);

	let lua_code = r#"
        local res = aip.file.list({
            globs = "*.txt",
            contains = { regex = "[unclosed_bracket" }
        })
        return res
    "#;

	// -- Exec
	let outcome = engine.exec(lua_code, context).await?;

	// -- Check
	assert!(outcome.result.is_err());
	let err_msg = outcome.result.unwrap_err().to_string();
	assert!(err_msg.contains("INVALID_REGEX"));

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

#[tokio::test]
async fn test_aiprog_file_write_and_append_flow() -> TestResult {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let workspace = simple_fs::SPath::from_std_path(tmp.path())?;
	let dir_context = DirContext::from_base_dir(workspace)?;

	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let mut context = RunningContext::default();
	context.insert(dir_context);

	let lua_code = r#"
        local write_res = aip.file.write({
            path = "output/greeting.txt",
            content = "  hello world\n",
            trim_start = true,
            single_trailing_newline = true
        })

        local append_res = aip.file.append({
            path = "output/greeting.txt",
            content = "second line\n"
        })

        local read_res = aip.file.read({
            path = "output/greeting.txt"
        })

        return {
            write_name = write_res.name,
            append_name = append_res.name,
            content = read_res.content
        }
    "#;

	// -- Exec
	let result = engine.exec(lua_code, context).await?.result?;

	// -- Check
	assert_eq!(result["write_name"], json!("greeting.txt"));
	assert_eq!(result["append_name"], json!("greeting.txt"));
	assert_eq!(result["content"], json!("hello world\nsecond line\n"));

	Ok(())
}

#[tokio::test]
async fn test_aiprog_file_copy_move_delete_flow() -> TestResult {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let workspace = simple_fs::SPath::from_std_path(tmp.path())?;
	let dir_context = DirContext::from_base_dir(workspace)?;

	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let mut context = RunningContext::default();
	context.insert(dir_context);

	let lua_code = r#"
        aip.file.write({ path = "source.txt", content = "original data" })
        local copy_res = aip.file.copy({ src = "source.txt", dest = "backup/copy.txt" })
        local move_res = aip.file.move({ src = "backup/copy.txt", dest = "moved/final.txt" })
        local deleted = aip.file.delete({ path = "source.txt" })
        local exists_source = aip.file.exists({ path = "source.txt" })
        local exists_moved = aip.file.exists({ path = "moved/final.txt" })

        return {
            copy_name = copy_res.name,
            move_name = move_res.name,
            deleted = deleted,
            exists_source = exists_source,
            exists_moved = exists_moved
        }
    "#;

	// -- Exec
	let result = engine.exec(lua_code, context).await?.result?;

	// -- Check
	assert_eq!(result["copy_name"], json!("copy.txt"));
	assert_eq!(result["move_name"], json!("final.txt"));
	assert_eq!(result["deleted"], json!(true));
	assert_eq!(result["exists_source"], json!(false));
	assert_eq!(result["exists_moved"], json!(true));

	Ok(())
}

#[tokio::test]
async fn test_aiprog_file_ensure_exists_and_ensure_dir() -> TestResult {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let workspace = simple_fs::SPath::from_std_path(tmp.path())?;
	let dir_context = DirContext::from_base_dir(workspace)?;

	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let mut context = RunningContext::default();
	context.insert(dir_context);

	let lua_code = r#"
        local dir_created = aip.file.ensure_dir({ path = "deeply/nested/dir" })
        local dir_again = aip.file.ensure_dir({ path = "deeply/nested/dir" })

        local file_res = aip.file.ensure_exists({
            path = "deeply/nested/dir/init.txt",
            content = "default config"
        })

        local read_res = aip.file.read({ path = "deeply/nested/dir/init.txt" })

        return {
            dir_created = dir_created,
            dir_again = dir_again,
            file_name = file_res.name,
            content = read_res.content
        }
    "#;

	// -- Exec
	let result = engine.exec(lua_code, context).await?.result?;

	// -- Check
	assert_eq!(result["dir_created"], json!(true));
	assert_eq!(result["dir_again"], json!(false));
	assert_eq!(result["file_name"], json!("init.txt"));
	assert_eq!(result["content"], json!("default config"));

	Ok(())
}

#[tokio::test]
async fn test_aiprog_file_list_default_excludes_and_overrides() -> TestResult {
	// -- Setup & Fixtures
	let tmp = TempDir::new()?;
	let workspace = simple_fs::SPath::from_std_path(tmp.path())?;
	std::fs::create_dir_all(tmp.path().join("target/debug"))?;
	std::fs::create_dir_all(tmp.path().join("node_modules/lib"))?;
	std::fs::create_dir_all(tmp.path().join(".git"))?;
	std::fs::create_dir_all(tmp.path().join("src"))?;

	std::fs::write(tmp.path().join("target/debug/binary.txt"), "target content")?;
	std::fs::write(tmp.path().join("node_modules/lib/dep.js"), "module content")?;
	std::fs::write(tmp.path().join(".git/config"), "git config")?;
	std::fs::write(tmp.path().join(".DS_Store"), "junk")?;
	std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}")?;

	let dir_context = DirContext::from_base_dir(workspace)?;
	let engine = ScriptEngine::builder()
		.with_registry(AipRegistry::from_aip_modules()?)
		.build()?;

	let mut context = RunningContext::default();
	context.insert(dir_context);

	let lua_code = r#"
        local all_files = aip.file.list({ globs = "**/*" })
        local target_files = aip.file.list({ globs = "target/**/*" })
        local custom_excluded = aip.file.list({ globs = { "**/*", "!**/*.rs" } })

        return {
            all_count = #all_files,
            all_first = all_files[1].name,
            target_count = #target_files,
            target_first = target_files[1].name,
            custom_count = #custom_excluded
        }
    "#;

	// -- Exec
	let result = engine.exec(lua_code, context).await?.result?;

	// -- Check
	assert_eq!(result["all_count"], json!(1));
	assert_eq!(result["all_first"], json!("main.rs"));
	assert_eq!(result["target_count"], json!(1));
	assert_eq!(result["target_first"], json!("binary.txt"));
	assert_eq!(result["custom_count"], json!(0));

	Ok(())
}
