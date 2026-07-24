use aiprog::{AipRegistry, EngineTemplate, RunningContext};
use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};

const MODEL: &str = "gpt-5.4-mini";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// 1. Build registry from built-in modules.
	let registry = AipRegistry::from_aip_modules()?;
	let engine = EngineTemplate::builder().with_registry(registry).build()?;

	// 2. Generate documentation.
	let doc = engine.generate_doc()?;

	// 3. Build the prompt.
	let task = "Read the JSON file 'tests/data/json/01-simple.json'\n\
		and return a string like:\n\
	  'Here is the num property value: ..., and the extra.values [does | does not] include 'five'\n\n";
	let prompt = format!(
		"Your goal is to write aiprog Lua scripts for the given task.

Below is the documentation of the available aiprog Lua API:\n\n\
<AIPROG_LUA_APIS>
{doc}
</AIPROG_LUA_APIS>

Your task:
{task}

Write a Lua script that accomplishes the task. Use only the APIs documented above.
Return ONLY the Lua code inside a fenced code block with the language tag `lua`.
Do not include any other text."
	);

	// 4. Call genai.
	let client = Client::builder().build();
	let chat_req = ChatRequest::new(vec![ChatMessage::user(&prompt)]);
	let chat_res = client.exec_chat(MODEL, chat_req, None).await?;
	let ai_text = chat_res.first_text().unwrap_or_default();

	println!("=== ai_text:\n{ai_text}\n");

	// 5. Extract Lua code block.
	let lua_code = extract_lua_block(ai_text)?;

	// 6. Execute with engine.
	let res = engine
		.exec(&lua_code, RunningContext::default())
		.await?
		.result?;

	let res = res.as_str().ok_or("Lua code should have returned a string")?;

	// 7. Print result.
	println!("=== Result:\n{res}");

	Ok(())
}

fn extract_lua_block(text: &str) -> core::result::Result<String, Box<dyn std::error::Error>> {
	let start_marker = "```lua";
	let end_marker = "```";
	let start = text.find(start_marker).ok_or("No luac code block found")?;
	let after_start = &text[start + start_marker.len()..];
	let after_newline = after_start.trim_start_matches(['\n', '\r']);
	let end = after_newline.find(end_marker).ok_or("Unclosed ```lua block in AI response")?;
	let code = after_newline[..end].trim().to_string();
	Ok(code)
}
