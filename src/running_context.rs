use core::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct RunningContext {
	values: HashMap<TypeId, Box<dyn Any + Send + Sync + 'static>>,
}

impl fmt::Debug for RunningContext {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("RunningContext")
			.field("value_count", &self.values.len())
			.finish()
	}
}

impl RunningContext {
	pub fn insert<T>(&mut self, value: T) -> Option<T>
	where
		T: Any + Send + Sync + 'static,
	{
		self.values
			.insert(TypeId::of::<T>(), Box::new(value))
			.and_then(downcast_owned::<T>)
	}

	pub fn get<T>(&self) -> Option<&T>
	where
		T: Any + Send + Sync + 'static,
	{
		self.values.get(&TypeId::of::<T>()).and_then(|value| value.downcast_ref::<T>())
	}

	pub fn get_mut<T>(&mut self) -> Option<&mut T>
	where
		T: Any + Send + Sync + 'static,
	{
		self.values
			.get_mut(&TypeId::of::<T>())
			.and_then(|value| value.downcast_mut::<T>())
	}

	pub fn remove<T>(&mut self) -> Option<T>
	where
		T: Any + Send + Sync + 'static,
	{
		self.values.remove(&TypeId::of::<T>()).and_then(downcast_owned::<T>)
	}
}

#[derive(Clone)]
pub struct HandlerCallContext {
	running: RunningContextHandle,
}

impl HandlerCallContext {
	pub fn with<T, R>(&self, action: impl FnOnce(&T) -> R) -> core::result::Result<R, ContextAccessError>
	where
		T: Any + Send + Sync + 'static,
	{
		self.running.with(action)
	}

	pub fn with_mut<T, R>(&self, action: impl FnOnce(&mut T) -> R) -> core::result::Result<R, ContextAccessError>
	where
		T: Any + Send + Sync + 'static,
	{
		self.running.with_mut(action)
	}

	pub(crate) fn new(running: RunningContextHandle) -> Self {
		Self { running }
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextAccessError {
	MissingValue { type_name: &'static str },
	LockPoisoned,
}

impl fmt::Display for ContextAccessError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::MissingValue { type_name } => {
				write!(f, "Running context does not contain a value of type '{type_name}'")
			}
			Self::LockPoisoned => f.write_str("Running context lock is poisoned"),
		}
	}
}

impl std::error::Error for ContextAccessError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextRecoveryError {
	OutstandingHandles,
	LockPoisoned,
}

impl fmt::Display for ContextRecoveryError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::OutstandingHandles => f.write_str("Running context cannot be recovered while handles remain"),
			Self::LockPoisoned => f.write_str("Running context lock is poisoned"),
		}
	}
}

impl std::error::Error for ContextRecoveryError {}

// region:    --- Support

#[derive(Clone)]
pub(crate) struct RunningContextHandle {
	inner: Arc<Mutex<RunningContext>>,
}

impl RunningContextHandle {
	pub(crate) fn new(context: RunningContext) -> Self {
		Self {
			inner: Arc::new(Mutex::new(context)),
		}
	}

	pub(crate) fn with<T, R>(&self, action: impl FnOnce(&T) -> R) -> core::result::Result<R, ContextAccessError>
	where
		T: Any + Send + Sync + 'static,
	{
		let context = self.inner.lock().map_err(|_| ContextAccessError::LockPoisoned)?;
		let value = context.get::<T>().ok_or(ContextAccessError::MissingValue {
			type_name: core::any::type_name::<T>(),
		})?;

		Ok(action(value))
	}

	pub(crate) fn with_mut<T, R>(&self, action: impl FnOnce(&mut T) -> R) -> core::result::Result<R, ContextAccessError>
	where
		T: Any + Send + Sync + 'static,
	{
		let mut context = self.inner.lock().map_err(|_| ContextAccessError::LockPoisoned)?;
		let value = context.get_mut::<T>().ok_or(ContextAccessError::MissingValue {
			type_name: core::any::type_name::<T>(),
		})?;

		Ok(action(value))
	}

	pub(crate) fn recover(self) -> core::result::Result<RunningContext, ContextRecoveryError> {
		Arc::try_unwrap(self.inner)
			.map_err(|_| ContextRecoveryError::OutstandingHandles)?
			.into_inner()
			.map_err(|_| ContextRecoveryError::LockPoisoned)
	}
}

fn downcast_owned<T>(value: Box<dyn Any + Send + Sync + 'static>) -> Option<T>
where
	T: Any + Send + Sync + 'static,
{
	value.downcast::<T>().ok().map(|value| *value)
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_running_context_insert_replaces_same_type() -> Result<()> {
		// -- Setup & Fixtures
		let mut context = RunningContext::default();

		// -- Exec
		let initial = context.insert(String::from("initial"));
		let replaced = context.insert(String::from("replacement"));
		let current = context.get::<String>().ok_or("Should contain a String value")?;

		// -- Check
		assert!(initial.is_none());
		assert_eq!(replaced.as_deref(), Some("initial"));
		assert_eq!(current, "replacement");
		assert!(context.get::<u32>().is_none());

		Ok(())
	}

	#[test]
	fn test_running_context_get_mut_updates_value() -> Result<()> {
		// -- Setup & Fixtures
		let mut context = RunningContext::default();
		context.insert::<u32>(10);

		// -- Exec
		let value = context.get_mut::<u32>().ok_or("Should contain a mutable u32 value")?;
		*value += 5;

		// -- Check
		assert_eq!(context.get::<u32>(), Some(&15));

		Ok(())
	}

	#[test]
	fn test_running_context_remove_returns_owned_value() -> Result<()> {
		// -- Setup & Fixtures
		let mut context = RunningContext::default();
		context.insert(String::from("stored"));

		// -- Exec
		let removed = context.remove::<String>();
		let remaining = context.get::<String>();

		// -- Check
		assert_eq!(removed.as_deref(), Some("stored"));
		assert!(remaining.is_none());

		Ok(())
	}

	#[test]
	fn test_running_context_handler_call_context_reports_missing_value() -> Result<()> {
		// -- Setup & Fixtures
		let handle = RunningContextHandle::new(RunningContext::default());
		let call_context = HandlerCallContext::new(handle);

		// -- Exec
		let error = call_context
			.with::<String, _>(String::len)
			.err()
			.ok_or("Should return a missing context value error")?;

		// -- Check
		assert_eq!(
			error,
			ContextAccessError::MissingValue {
				type_name: core::any::type_name::<String>(),
			}
		);

		Ok(())
	}

	#[test]
	fn test_running_context_handle_recover_success() -> Result<()> {
		// -- Setup & Fixtures
		let mut context = RunningContext::default();
		context.insert::<u32>(42);
		let handle = RunningContextHandle::new(context);

		// -- Exec
		let recovered = handle.recover()?;
		let value = recovered.get::<u32>().ok_or("Should recover the stored u32 value")?;

		// -- Check
		assert_eq!(*value, 42);

		Ok(())
	}

	#[test]
	fn test_running_context_handle_recover_outstanding_handle() -> Result<()> {
		// -- Setup & Fixtures
		let handle = RunningContextHandle::new(RunningContext::default());
		let outstanding = handle.clone();

		// -- Exec
		let error = handle
			.recover()
			.err()
			.ok_or("Should reject recovery while another handle exists")?;

		// -- Check
		assert_eq!(error, ContextRecoveryError::OutstandingHandles);
		drop(outstanding);

		Ok(())
	}
}

// endregion: --- Tests
