use std::future::Future;
use std::pin::Pin;

use mlua::{Lua, Value};
use schemars::Schema;

use super::registry_types::AipFnKind;

pub type LuaSyncClosure = Box<dyn Fn(&Lua, Value) -> mlua::Result<Value> + Send + Sync>;

pub type LuaAsyncClosure =
	Box<dyn Fn(&Lua, Value) -> Pin<Box<dyn Future<Output = mlua::Result<serde_json::Value>>>> + Send + Sync>;

#[doc(hidden)]
pub struct RegistryEntry {
	pub path: String,
	pub kind: AipFnKind,
	pub handler: AipHandlerClosure,
	pub params_schema: Schema,
	pub output_schema: Schema,
	pub error_schema: Schema,
	pub description: Option<String>,
	pub title: Option<String>,
}

pub enum AipHandlerClosure {
	Sync(LuaSyncClosure),
	Async(LuaAsyncClosure),
}
