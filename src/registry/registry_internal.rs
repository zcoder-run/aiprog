use std::future::Future;
use std::pin::Pin;

use mlua::{Lua, Value};
use schemars::Schema;

use super::registry_types::AipFnKind;

pub(crate) type LuaSyncClosure = Box<dyn Fn(&Lua, Value) -> mlua::Result<Value> + Send + Sync>;

pub(crate) type LuaAsyncClosure =
	Box<dyn Fn(&Lua, Value) -> Pin<Box<dyn Future<Output = mlua::Result<serde_json::Value>> + Send>> + Send + Sync>;

pub(crate) struct RegistryEntry {
	pub path: String,
	pub kind: AipFnKind,
	pub handler: AipHandlerClosure,
	pub params_schema: Schema,
	pub response_schema: Schema,
	pub error_schema: Schema,
}

pub(crate) enum AipHandlerClosure {
	Sync(LuaSyncClosure),
	Async(LuaAsyncClosure),
}
