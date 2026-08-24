use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct LuaRuntimePolicy {
	std_lib_policy: LuaStdLibPolicy,
	limits: LuaExecutionLimits,
}

#[derive(Debug, Clone)]
pub struct LuaStdLibPolicy {
	pub base: bool,
	pub coroutine: bool,
	pub math: bool,
	pub string: bool,
	pub table: bool,
	pub utf8: bool,
	pub package: bool,
	pub io: bool,
	pub os: bool,
	pub debug: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LuaExecutionLimits {
	pub max_memory_bytes: Option<usize>,
	pub max_instructions: Option<u64>,
	pub wall_clock_timeout: Option<Duration>,
}

impl LuaRuntimePolicy {
	pub fn with_std_lib_policy(mut self, policy: LuaStdLibPolicy) -> Self {
		self.std_lib_policy = policy;
		self
	}

	pub fn with_limits(mut self, limits: LuaExecutionLimits) -> Self {
		self.limits = limits;
		self
	}

	pub fn std_lib_policy(&self) -> &LuaStdLibPolicy {
		&self.std_lib_policy
	}

	pub fn limits(&self) -> &LuaExecutionLimits {
		&self.limits
	}
}

impl Default for LuaStdLibPolicy {
	fn default() -> Self {
		Self {
			base: true,
			coroutine: false,
			math: true,
			string: true,
			table: true,
			utf8: true,
			package: false,
			io: false,
			os: false,
			debug: false,
		}
	}
}

impl LuaStdLibPolicy {
	pub fn with_base(mut self, enabled: bool) -> Self {
		self.base = enabled;
		self
	}

	pub fn with_coroutine(mut self, enabled: bool) -> Self {
		self.coroutine = enabled;
		self
	}

	pub fn with_math(mut self, enabled: bool) -> Self {
		self.math = enabled;
		self
	}

	pub fn with_string(mut self, enabled: bool) -> Self {
		self.string = enabled;
		self
	}

	pub fn with_table(mut self, enabled: bool) -> Self {
		self.table = enabled;
		self
	}

	pub fn with_utf8(mut self, enabled: bool) -> Self {
		self.utf8 = enabled;
		self
	}

	pub fn with_package(mut self, enabled: bool) -> Self {
		self.package = enabled;
		self
	}

	pub fn with_io(mut self, enabled: bool) -> Self {
		self.io = enabled;
		self
	}

	pub fn with_os(mut self, enabled: bool) -> Self {
		self.os = enabled;
		self
	}

	pub fn with_debug(mut self, enabled: bool) -> Self {
		self.debug = enabled;
		self
	}
}

impl LuaExecutionLimits {
	pub fn with_max_memory_bytes(mut self, max_memory_bytes: usize) -> Self {
		self.max_memory_bytes = Some(max_memory_bytes);
		self
	}

	pub fn with_max_instructions(mut self, max_instructions: u64) -> Self {
		self.max_instructions = Some(max_instructions);
		self
	}

	pub fn with_wall_clock_timeout(mut self, wall_clock_timeout: Duration) -> Self {
		self.wall_clock_timeout = Some(wall_clock_timeout);
		self
	}

	pub fn max_memory_bytes(&self) -> Option<usize> {
		self.max_memory_bytes
	}

	pub fn max_instructions(&self) -> Option<u64> {
		self.max_instructions
	}

	pub fn wall_clock_timeout(&self) -> Option<Duration> {
		self.wall_clock_timeout
	}
}
