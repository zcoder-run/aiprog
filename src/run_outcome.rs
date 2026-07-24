use crate::RunningContext;

/// The result of one script execution together with its recovered context.
#[derive(Debug)]
pub struct RunOutcome<T, E = crate::Error> {
	pub result: core::result::Result<T, E>,
	pub context: RunningContext,
}

impl<T, E> RunOutcome<T, E> {
	pub fn new(result: core::result::Result<T, E>, context: RunningContext) -> Self {
		Self { result, context }
	}

	pub fn into_parts(self) -> (core::result::Result<T, E>, RunningContext) {
		(self.result, self.context)
	}
}
