use crate::args::*;
use crate::mcp::*;
use serde_json::json;
use std::io::{self, BufRead, Write};

pub fn run_stdio_server() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::error(None, -32700, format!("Parse error: {}", e));
                send_response(&mut stdout, &resp)?;
                continue;
            }
        };
        
        let resp = handle_request(req)?;
        send_response(&mut stdout, &resp)?;
    }
    
    Ok(())
}

fn send_response(stdout: &mut io::Stdout, resp: &JsonRpcResponse) -> io::Result<()> {
    let json = serde_json::to_string(resp)?;
    writeln!(stdout, "{}", json)?;
    stdout.flush()?;
    Ok(())
}

pub fn handle_request(req: JsonRpcRequest) -> anyhow::Result<JsonRpcResponse> {
    match req.method.as_str() {
        "initialize" => {
            let result = InitializeResult {
                protocol_version: "2024-11-05".into(),
                capabilities: ServerCapabilities::default(),
                server_info: ServerInfo { name: "gradience-mcp".into(), version: "0.1.0".into() },
            };
            Ok(JsonRpcResponse::success(req.id, serde_json::to_value(result)?))
        }
        "notifications/initialized" => {
            Ok(JsonRpcResponse::success(req.id, json!({})))
        }
        "tools/list" => {
            let tools = vec![
                Tool::with_schema::<SignTxArgs>("sign_transaction", "Sign a blockchain transaction"),
                Tool::with_schema::<GetBalanceArgs>("get_balance", "Get wallet balance"),
                Tool::with_schema::<SwapArgs>("swap", "Execute DEX swap"),
                Tool::with_schema::<PayArgs>("pay", "Execute x402 payment"),
                Tool::with_schema::<LlmGenerateArgs>("llm_generate", "Generate text via AI Gateway"),
                Tool::with_schema::<AiBalanceArgs>("ai_balance", "Query AI Gateway balance"),
                Tool::with_schema::<AiModelsArgs>("ai_models", "List available LLM models and pricing"),
            ];
            let result = ToolsListResult { tools };
            Ok(JsonRpcResponse::success(req.id, serde_json::to_value(result)?))
        }
        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));

            let result = match name {
                "sign_transaction" => {
                    let a: SignTxArgs = match serde_json::from_value(args) {
                        Ok(v) => v,
                        Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, format!("invalid args: {}", e))),
                    };
                    match crate::tools::handle_sign_transaction(a) {
                        Ok(v) => v,
                        Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, e.to_string())),
                    }
                }
                "get_balance" => {
                    let a: GetBalanceArgs = match serde_json::from_value(args) {
                        Ok(v) => v,
                        Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, format!("invalid args: {}", e))),
                    };
                    match crate::tools::handle_get_balance(a) {
                        Ok(v) => v,
                        Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, e.to_string())),
                    }
                }
                "swap" => {
                    let a: SwapArgs = match serde_json::from_value(args) {
                        Ok(v) => v,
                        Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, format!("invalid args: {}", e))),
                    };
                    match crate::tools::handle_swap(a) {
                        Ok(v) => v,
                        Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, e.to_string())),
                    }
                }
                "pay" => {
                    let a: PayArgs = match serde_json::from_value(args) {
                        Ok(v) => v,
                        Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, format!("invalid args: {}", e))),
                    };
                    match crate::tools::handle_pay(a) {
                        Ok(v) => v,
                        Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, e.to_string())),
                    }
                }
                "llm_generate" => {
                    let a: LlmGenerateArgs = match serde_json::from_value(args) {
                        Ok(v) => v,
                        Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, format!("invalid args: {}", e))),
                    };
                    match crate::tools::handle_llm_generate(a) {
                        Ok(v) => v,
                        Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, e.to_string())),
                    }
                }
                "ai_balance" => {
                    let a: AiBalanceArgs = match serde_json::from_value(args) {
                        Ok(v) => v,
                        Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, format!("invalid args: {}", e))),
                    };
                    match crate::tools::handle_ai_balance(a) {
                        Ok(v) => v,
                        Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, e.to_string())),
                    }
                }
                "ai_models" => match crate::tools::handle_ai_models() {
                    Ok(v) => v,
                    Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, e.to_string())),
                },
                _ => return Ok(JsonRpcResponse::error(req.id, -32601, format!("Unknown tool: {}", name))),
            };

            Ok(JsonRpcResponse::success(req.id, json!({
                "content": [{ "type": "text", "text": result.to_string() }]
            })))
        }
        _ => Ok(JsonRpcResponse::error(req.id, -32601, format!("Method not found: {}", req.method))),
    }
}
