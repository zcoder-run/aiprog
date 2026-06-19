// region:    --- Tests

#![allow(unused_imports)] // For test types

use crate::{AipFromLua, AipIntoLua, AipParams};
use aiprog::{
    mlua::Lua,
    script::{AipFromLua as _, AipIntoLua as _},
    ScriptError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

// region:    --- Test types

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema, AipFromLua, AipIntoLua, AipParams)]
struct TestParams {
    text: String,
    count: i64,
    flag: bool,
    maybe: Option<String>,
    items: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema, AipFromLua, AipIntoLua)]
struct NamedWrapper<T> {
    inner: T,
}

// endregion: --- Test types

// region:    --- Roundtrip tests

#[test]
fn test_roundtrip_simple() -> Result<()> {
    let lua = Lua::new();
    let original = TestParams {
        text: "hello".into(),
        count: 42,
        flag: true,
        maybe: Some("world".into()),
        items: vec!["a".into(), "b".into()],
    };
    let lua_val = original.into_lua(&lua)?;
    let back = TestParams::from_lua(&lua, lua_val)?;
    assert_eq!(back, original);
    Ok(())
}

#[test]
fn test_roundtrip_option_none() -> Result<()> {
    let lua = Lua::new();
    let original = TestParams {
        text: "".into(),
        count: 0,
        flag: false,
        maybe: None,
        items: vec![],
    };
    let lua_val = original.into_lua(&lua)?;
    let back = TestParams::from_lua(&lua, lua_val)?;
    assert_eq!(back.maybe, None);
    Ok(())
}

#[test]
fn test_roundtrip_generic() -> Result<()> {
    let lua = Lua::new();
    let original = NamedWrapper { inner: 123i64 };
    let lua_val = original.into_lua(&lua)?;
    let back: NamedWrapper<i64> = NamedWrapper::from_lua(&lua, lua_val)?;
    assert_eq!(back, original);
    Ok(())
}

// region:    --- Single-field tuple delegation test

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema, AipIntoLua)]
struct SingleFieldWrapper(i64);

#[test]
fn test_single_field_tuple_into_lua_delegates_directly() -> Result<()> {
    let lua = Lua::new();
    let original = SingleFieldWrapper(42);
    let lua_val = original.into_lua(&lua)?;
    // Delegation should produce the inner i64 directly, not a Lua table wrapping it.
    assert_eq!(lua_val, aiprog::mlua::Value::Integer(42));
    Ok(())
}

// endregion: --- Single-field tuple delegation test

// endregion: --- Roundtrip tests

// region:    --- Error cases

#[test]
fn test_from_lua_wrong_type() -> Result<()> {
    let lua = Lua::new();
    let wrong_lua_val = lua.create_string("not a table")?;
    let err = TestParams::from_lua(&lua, aiprog::mlua::Value::String(wrong_lua_val)).unwrap_err();
    match err {
        ScriptError::Custom(msg) => {
            assert!(
                msg.contains("Invalid params") || msg.contains("deserialization"),
                "unexpected error message: {msg}"
            );
        }
    }
    Ok(())
}

// endregion: --- Error cases

// region:    --- Marker trait check

fn assert_aip_params<P: AipParams>() {}

#[test]
fn test_aip_params_marker() {
    assert_aip_params::<TestParams>();
}

// endregion: --- Marker trait check

// region:    --- Support

// endregion: --- Support

// endregion: --- Tests
