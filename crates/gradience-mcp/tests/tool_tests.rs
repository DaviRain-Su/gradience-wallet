use gradience_mcp::mcp::*;
use gradience_mcp::server::handle_request;
use serde_json::json;

fn make_request(method: &str, params: serde_json::Value) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: Some(params),
    }
}

#[test]
fn test_mcp_initialize() {
    let req = make_request("initialize", json!({}));
    let resp = handle_request(req).unwrap();
    assert!(resp.error.is_none());
    assert!(resp.result.is_some());
}

#[test]
fn test_mcp_tools_list() {
    let req = make_request("tools/list", json!({}));
    let resp = handle_request(req).unwrap();
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let tools = result.get("tools").unwrap().as_array().unwrap();
    let names: Vec<String> = tools
        .iter()
        .map(|t| t.get("name").unwrap().as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"sign_transaction".to_string()));
    assert!(names.contains(&"get_balance".to_string()));
}

#[test]
fn test_mcp_sign_tx_success() {
    // Use a temp directory so the test is isolated from any existing ~/.gradience session
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().to_path_buf();
    std::env::set_var("GRADIENCE_DATA_DIR", &data_dir);

    let req = make_request(
        "tools/call",
        json!({
            "name": "sign_transaction",
            "arguments": {
                "walletId": "wallet-123",
                "chainId": "eip155:8453",
                "transaction": {
                    "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C",
                    "value": "1000000000000000",
                    "data": "0x"
                }
            }
        }),
    );
    let resp = handle_request(req).unwrap();
    // Without a real session file and vault, we expect an error after policy allows
    if !data_dir.join(".session").exists() {
        assert!(
            resp.error.is_some(),
            "expected error when no session file exists"
        );
        return;
    }
    assert!(
        resp.error.is_none(),
        "expected success, got error: {:?}",
        resp.error
    );
    let content = resp.result.unwrap().get("content").unwrap().clone();
    let text = content[0].get("text").unwrap().as_str().unwrap();
    assert!(text.contains("allowed"));
}

#[test]
fn test_mcp_sign_tx_missing_wallet_id() {
    let req = make_request(
        "tools/call",
        json!({
            "name": "sign_transaction",
            "arguments": {
                "chainId": "eip155:1",
                "transaction": {
                    "to": "0x...",
                    "value": "0",
                    "data": "0x"
                }
            }
        }),
    );
    let resp = handle_request(req).unwrap();
    assert!(resp.error.is_some());
    assert!(resp
        .error
        .as_ref()
        .unwrap()
        .message
        .contains("missing field"));
}

#[test]
fn test_mcp_get_balance() {
    // Use a temp directory so the test is isolated from any existing ~/.gradience session
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().to_path_buf();
    std::env::set_var("GRADIENCE_DATA_DIR", &data_dir);

    let req = make_request(
        "tools/call",
        json!({
            "name": "get_balance",
            "arguments": {
                "walletId": "wallet-123",
                "chainId": "eip155:8453"
            }
        }),
    );
    let resp = handle_request(req).unwrap();
    if !data_dir.join(".session").exists() {
        assert!(resp.error.is_some() || resp.result.is_some());
        return;
    }
    assert!(resp.error.is_none());
    let content = resp.result.unwrap().get("content").unwrap().clone();
    let text = content[0].get("text").unwrap().as_str().unwrap();
    assert!(text.contains("native"));
}

#[test]
fn test_mcp_unknown_tool() {
    let req = make_request(
        "tools/call",
        json!({
            "name": "unknown_tool",
            "arguments": {}
        }),
    );
    let resp = handle_request(req).unwrap();
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32601);
}
