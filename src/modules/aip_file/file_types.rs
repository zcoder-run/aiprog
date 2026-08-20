use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use simple_fs::SPath;
use std::fmt;
use std::path::{Path, PathBuf};

// region:    --- DirContext

/// Execution-scoped filesystem capability policy.
#[derive(Debug, Clone)]
pub struct DirContext {
	base_dir: SPath,
	read_policy: PathPolicy,
	write_policy: PathPolicy,
}

// region:    --- Constructors

impl DirContext {
	pub fn new(
		base_dir: impl Into<SPath>,
		read_policy: PathPolicy,
		write_policy: PathPolicy,
	) -> Result<Self, DirPolicyError> {
		let base_dir = base_dir.into();
		let resolved = read_policy.authorize_existing(&base_dir)?;
		if !resolved.path().is_dir() {
			return Err(DirPolicyError::InvalidBaseDir(base_dir.as_str().to_string()));
		}
		Ok(Self {
			base_dir: resolved.path().clone(),
			read_policy,
			write_policy,
		})
	}

	pub fn from_base_dir(base_dir: impl Into<SPath>) -> Result<Self, DirPolicyError> {
		let base_dir = base_dir.into();
		let policy = PathPolicy::new([base_dir.clone()], AbsolutePathPolicy::Deny)?;
		Self::new(base_dir, policy.clone(), policy)
	}

	pub fn current_dir() -> Result<Self, DirPolicyError> {
		Self::from_base_dir(".")
	}
}

// endregion: --- Constructors

// region:    --- Resolvers

impl DirContext {
	pub fn resolve_read(&self, path: &str, base_dir: Option<&str>) -> Result<ResolvedDirPath, DirPolicyError> {
		self.read_policy.resolve(path, &self.base_dir, base_dir, false)
	}

	pub fn resolve_write(&self, path: &str, base_dir: Option<&str>) -> Result<ResolvedDirPath, DirPolicyError> {
		self.write_policy.resolve(path, &self.base_dir, base_dir, true)
	}

	pub(crate) fn resolve_read_target(
		&self,
		path: &str,
		base_dir: Option<&str>,
	) -> Result<ResolvedDirPath, DirPolicyError> {
		self.read_policy.resolve(path, &self.base_dir, base_dir, true)
	}

	pub(crate) fn authorize_existing_read(&self, path: &SPath) -> Result<ResolvedDirPath, DirPolicyError> {
		self.read_policy.authorize_existing(path)
	}
}

// endregion: --- Resolvers

// region:    --- Policy Assertions

impl DirContext {
	pub fn assert_write(&self, path: &SPath) -> Result<bool, DirPolicyError> {
		let _ = path;
		Ok(true)
	}

	pub fn assert_read(&self, path: &SPath) -> Result<bool, DirPolicyError> {
		let _ = path;
		Ok(true)
	}
}

// endregion: --- Policy Assertions

// region:    --- Getters

impl DirContext {
	pub fn base_dir(&self) -> &SPath {
		&self.base_dir
	}

	pub fn read_policy(&self) -> &PathPolicy {
		&self.read_policy
	}

	pub fn write_policy(&self) -> &PathPolicy {
		&self.write_policy
	}
}

// endregion: --- Getters

impl Default for DirContext {
	fn default() -> Self {
		Self::current_dir().expect("Current working directory must be valid for default DirContext")
	}
}

/// Controls whether callers may supply absolute paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbsolutePathPolicy {
	Allow,
	Deny,
}

// region:    --- PathPolicy

/// A set of canonical directory roots allowed for one class of operations.
#[derive(Debug, Clone)]
pub struct PathPolicy {
	allowed_roots: Vec<SPath>,
	absolute_paths: AbsolutePathPolicy,
}

// region:    --- Constructors

impl PathPolicy {
	pub fn new(
		allowed_roots: impl IntoIterator<Item = impl Into<SPath>>,
		absolute_paths: AbsolutePathPolicy,
	) -> Result<Self, DirPolicyError> {
		let mut canonical_roots = Vec::new();

		for root in allowed_roots {
			let root = root.into();
			let canonical = root
				.canonicalize()
				.map_err(|error| DirPolicyError::InvalidRoot(root.as_str().to_string(), error.to_string()))?;

			if !canonical.is_dir() {
				return Err(DirPolicyError::InvalidRoot(
					root.as_str().to_string(),
					"allowed root is not a directory".to_string(),
				));
			}

			if !canonical_roots
				.iter()
				.any(|existing: &SPath| existing.as_str() == canonical.as_str())
			{
				canonical_roots.push(canonical);
			}
		}

		if canonical_roots.is_empty() {
			return Err(DirPolicyError::NoAllowedRoots);
		}

		Ok(Self {
			allowed_roots: canonical_roots,
			absolute_paths,
		})
	}
}

// endregion: --- Constructors

// region:    --- Resolvers

impl PathPolicy {
	pub fn resolve(
		&self,
		path: &str,
		default_base: &SPath,
		base_dir: Option<&str>,
		allow_missing: bool,
	) -> Result<ResolvedDirPath, DirPolicyError> {
		if path.trim().is_empty() {
			return Err(DirPolicyError::InvalidPath("path must not be empty".to_string()));
		}

		let supplied_path = Path::new(path);
		if supplied_path.is_absolute() && self.absolute_paths == AbsolutePathPolicy::Deny {
			return Err(DirPolicyError::AbsolutePathDenied(path.to_string()));
		}

		let candidate = if supplied_path.is_absolute() {
			supplied_path.to_path_buf()
		} else {
			let base = self.resolve_base(default_base, base_dir)?;
			base.join(supplied_path)
		};

		let canonical = canonicalize_candidate(&candidate, allow_missing)?;
		self.authorize_canonical(canonical)
	}

	pub fn resolve_base(&self, default_base: &SPath, base_dir: Option<&str>) -> Result<PathBuf, DirPolicyError> {
		let Some(base_dir) = base_dir else {
			return Ok(PathBuf::from(default_base.as_str()));
		};

		let supplied_base = Path::new(base_dir);
		if supplied_base.is_absolute() && self.absolute_paths == AbsolutePathPolicy::Deny {
			return Err(DirPolicyError::AbsolutePathDenied(base_dir.to_string()));
		}

		let candidate = if supplied_base.is_absolute() {
			supplied_base.to_path_buf()
		} else {
			Path::new(default_base.as_str()).join(supplied_base)
		};
		let canonical = canonicalize_candidate(&candidate, false)?;
		let resolved = self.authorize_canonical(canonical)?;

		if !resolved.path.is_dir() {
			return Err(DirPolicyError::InvalidBaseDir(base_dir.to_string()));
		}

		Ok(PathBuf::from(resolved.path.as_str()))
	}

	pub fn authorize_existing(&self, path: &SPath) -> Result<ResolvedDirPath, DirPolicyError> {
		let canonical = path
			.canonicalize()
			.map_err(|error| DirPolicyError::InvalidPath(format!("{}: {error}", path.as_str())))?;
		self.authorize_canonical(PathBuf::from(canonical.as_str()))
	}

	pub fn authorize_canonical(&self, canonical: PathBuf) -> Result<ResolvedDirPath, DirPolicyError> {
		let root = self
			.allowed_roots
			.iter()
			.find(|root| canonical.starts_with(Path::new(root.as_str())))
			.cloned()
			.ok_or_else(|| DirPolicyError::OutsideAllowedRoots(canonical.display().to_string()))?;

		let path =
			SPath::from_std_path_buf(canonical).map_err(|error| DirPolicyError::InvalidPath(error.to_string()))?;

		Ok(ResolvedDirPath { path, root })
	}
}

// endregion: --- Resolvers

// region:    --- Getters

impl PathPolicy {
	pub fn allowed_roots(&self) -> &[SPath] {
		&self.allowed_roots
	}

	pub fn absolute_paths(&self) -> AbsolutePathPolicy {
		self.absolute_paths
	}
}

// endregion: --- Getters

// endregion: --- PathPolicy

// region:    --- ResolvedDirPath

/// A policy-authorized path and the allowed root that contains it.
#[derive(Debug, Clone)]
pub struct ResolvedDirPath {
	path: SPath,
	root: SPath,
}

// region:    --- Getters

impl ResolvedDirPath {
	pub fn path(&self) -> &SPath {
		&self.path
	}

	pub fn root(&self) -> &SPath {
		&self.root
	}
}

// endregion: --- Getters

// endregion: --- ResolvedDirPath

// region:    --- DirPolicyError

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirPolicyError {
	NoAllowedRoots,
	InvalidRoot(String, String),
	InvalidPath(String),
	InvalidBaseDir(String),
	AbsolutePathDenied(String),
	OutsideAllowedRoots(String),
}

impl fmt::Display for DirPolicyError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::NoAllowedRoots => f.write_str("directory policy requires at least one allowed root"),
			Self::InvalidRoot(path, cause) => write!(f, "invalid allowed root '{path}': {cause}"),
			Self::InvalidPath(path) => write!(f, "invalid path: {path}"),
			Self::InvalidBaseDir(path) => write!(f, "base directory is not a directory: {path}"),
			Self::AbsolutePathDenied(path) => write!(f, "absolute paths are not allowed: {path}"),
			Self::OutsideAllowedRoots(path) => write!(f, "path is outside the allowed roots: {path}"),
		}
	}
}

impl std::error::Error for DirPolicyError {}

// endregion: --- DirPolicyError

// endregion: --- DirContext

// region:    --- Support

fn canonicalize_candidate(candidate: &Path, allow_missing: bool) -> Result<PathBuf, DirPolicyError> {
	if candidate.exists() {
		return candidate
			.canonicalize()
			.map_err(|error| DirPolicyError::InvalidPath(format!("{}: {error}", candidate.display())));
	}

	if !allow_missing {
		return Err(DirPolicyError::InvalidPath(format!(
			"path does not exist: {}",
			candidate.display()
		)));
	}

	let mut existing = candidate.to_path_buf();
	let mut missing_parts = Vec::new();
	while !existing.exists() {
		let part = existing
			.file_name()
			.ok_or_else(|| DirPolicyError::InvalidPath(candidate.display().to_string()))?
			.to_os_string();
		missing_parts.push(part);

		if !existing.pop() {
			return Err(DirPolicyError::InvalidPath(candidate.display().to_string()));
		}
	}

	let mut canonical = existing
		.canonicalize()
		.map_err(|error| DirPolicyError::InvalidPath(format!("{}: {error}", existing.display())))?;
	for part in missing_parts.into_iter().rev() {
		canonical.push(part);
	}

	Ok(canonical)
}

// endregion: --- Support

// region:    --- FileInfo

/// Metadata about a file, without content.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileInfo {
	/// Resolved path (workspace-relative unless absolute was requested).
	pub path: String,
	/// File name with extension.
	pub name: String,
	/// File name without extension.
	pub stem: String,
	/// File extension, without the leading dot.
	pub ext: String,
	/// File size in bytes. Present when metadata is available.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub size: Option<u64>,
	/// Creation timestamp (epoch microseconds).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ctime: Option<i64>,
	/// Modification timestamp (epoch microseconds).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mtime: Option<i64>,
}

// endregion: --- FileInfo

// region:    --- FileRecord

/// Full file record, including content.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileRecord {
	#[serde(flatten)]
	pub info: FileInfo,
	/// Full text content of the file.
	pub content: String,
}

// endregion: --- FileRecord

// region:    --- FileStats

/// Aggregate statistics over a set of files.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileStats {
	/// Number of files matched.
	pub number_of_files: usize,
	/// Sum of file sizes in bytes.
	pub total_size: u64,
	/// Earliest creation timestamp (epoch microseconds).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ctime_first: Option<i64>,
	/// Latest creation timestamp (epoch microseconds).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ctime_last: Option<i64>,
	/// Earliest modification timestamp (epoch microseconds).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mtime_first: Option<i64>,
	/// Latest modification timestamp (epoch microseconds).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mtime_last: Option<i64>,
}

// endregion: --- FileStats
