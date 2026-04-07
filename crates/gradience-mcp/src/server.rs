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
                crate::mcp::Tool {
                    name: "sign_transaction".into(),
                    description: "Sign a blockchain transaction".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "walletId": { "type": "string" },
                            "chainId": { "type": "string" },
                            "transaction": {
                                "type": "object",
                                "properties": {
                                    "to": { "type": "string" },
                                    "value": { "type": "string" },
                                    "data": { "type": "string" }
                                }
                            }
                        },
                        "required": ["walletId", "chainId", "transaction"]
                    }),
                },
                crate::mcp::Tool {
                    name: "get_balance".into(),
                    description: "Get wallet balance".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "walletId": { "type": "string" },
                            "chainId": { "type": "string" }
                        },
                        "required": ["walletId"]
                    }),
                },
            ];
            let result = ToolsListResult { tools };
            Ok(JsonRpcResponse::success(req.id, serde_json::to_value(result)?))
        }
        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            
            let result = match name {
                "sign_transaction" => match crate::tools::handle_sign_transaction(args) {
                    Ok(v) => v,
                    Err(e) => return Ok(JsonRpcResponse::error(req.id, -32000, e.to_string())),
                },
                "get_balance" => match crate::tools::handle_get_balance(args) {
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
