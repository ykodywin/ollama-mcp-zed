//! MCP stdio server exposing Ollama's hosted `web_search` and `web_fetch` as
//! MCP tools.
//!
//! Speaks JSON-RPC 2.0 over stdio (newline-delimited). The API key is read
//! from the `OLLAMA_API_KEY` environment variable (the process runs natively,
//! outside the Zed extension sandbox, so `std::env::var` works here).
//!
//! Endpoints (https://docs.ollama.com/capabilities/web-search):
//!   POST https://ollama.com/api/web_search  { "query", "max_results" }
//!   POST https://ollama.com/api/web_fetch   { "url" }
//! Both require `Authorization: Bearer <OLLAMA_API_KEY>`.

use std::env;
use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "ollama-search-mcp";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

const WEB_SEARCH_URL: &str = "https://ollama.com/api/web_search";
const WEB_FETCH_URL: &str = "https://ollama.com/api/web_fetch";

fn main() {
    let api_key = match env::var("OLLAMA_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => {
            eprintln!(
                "ollama-search-mcp: OLLAMA_API_KEY is not set. \
                 Get a free key at https://ollama.com/settings/keys"
            );
            std::process::exit(1);
        }
    };

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                // Can't recover a request id from invalid JSON; skip silently.
                eprintln!("ollama-search-mcp: ignoring unparseable line: {e}");
                continue;
            }
        };

        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = req.get("id").cloned();
        let params = req.get("params").cloned().unwrap_or(Value::Null);

        let response: Option<Value> = match method {
            "initialize" => Some(initialize_response(id)),
            "notifications/initialized" | "notifications/cancelled" => None,
            "ping" => Some(json!({ "jsonrpc": "2.0", "id": id, "result": {} })),
            "tools/list" => Some(tools_list_response(id)),
            "tools/call" => Some(tools_call_response(id, params, &api_key)),
            "" => None, // malformed, no method
            other => id
                .map(|id| error_response(id, -32601, &format!("Method not found: {other}"))),
        };

        if let Some(resp) = response {
            if writeln!(out, "{resp}").is_err() {
                break; // stdout closed (editor shutting down)
            }
            let _ = out.flush();
        }
    }
}

fn initialize_response(id: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": SERVER_NAME,
                "version": SERVER_VERSION
            }
        }
    })
}

fn tools_list_response(id: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": [
                {
                    "name": "web_search",
                    "description": "Search the web using Ollama's hosted search API. Returns a list of results with title, url and a content snippet.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "The search query." },
                            "max_results": {
                                "type": "integer",
                                "description": "Maximum results to return (1-10). Defaults to 5.",
                                "default": 5,
                                "minimum": 1,
                                "maximum": 10
                            }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "web_fetch",
                    "description": "Fetch and return the content of a web page at the given URL using Ollama's hosted fetch API.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "url": { "type": "string", "description": "The absolute URL to fetch." }
                        },
                        "required": ["url"]
                    }
                }
            ]
        }
    })
}

fn tools_call_response(id: Option<Value>, params: Value, api_key: &str) -> Value {
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let (text, is_error) = match name {
        "web_search" => call_web_search(&arguments, api_key),
        "web_fetch" => call_web_fetch(&arguments, api_key),
        other => (format!("Unknown tool: {other}"), true),
    };

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [ { "type": "text", "text": text } ],
            "isError": is_error
        }
    })
}

fn call_web_search(args: &Value, api_key: &str) -> (String, bool) {
    let query = match args.get("query").and_then(|q| q.as_str()) {
        Some(q) if !q.trim().is_empty() => q,
        _ => return ("Missing required argument 'query'".to_string(), true),
    };
    let max_results = args
        .get("max_results")
        .and_then(|m| m.as_i64())
        .unwrap_or(5)
        .clamp(1, 10) as i32;

    let body = json!({ "query": query, "max_results": max_results });
    match http_post(WEB_SEARCH_URL, &body, api_key) {
        Ok(resp) => (pretty(&resp), false),
        Err(e) => (e, true),
    }
}

fn call_web_fetch(args: &Value, api_key: &str) -> (String, bool) {
    let url = match args.get("url").and_then(|u| u.as_str()) {
        Some(u) if !u.trim().is_empty() => u,
        _ => return ("Missing required argument 'url'".to_string(), true),
    };

    let body = json!({ "url": url });
    match http_post(WEB_FETCH_URL, &body, api_key) {
        Ok(resp) => (pretty(&resp), false),
        Err(e) => (e, true),
    }
}

fn http_post(url: &str, body: &Value, api_key: &str) -> Result<Value, String> {
    match ureq::post(url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json")
        .send_json(body.clone())
    {
        Ok(r) => r
            .into_json::<Value>()
            .map_err(|e| format!("Failed to decode response: {e}")),
        Err(ureq::Error::Status(code, r)) => {
            let text = r.into_string().unwrap_or_default();
            Err(format!("HTTP {code} from {url}: {text}"))
        }
        Err(e) => Err(format!("Request to {url} failed: {e}")),
    }
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

fn error_response(id: Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}