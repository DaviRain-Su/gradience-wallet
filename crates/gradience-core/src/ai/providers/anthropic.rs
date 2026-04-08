use serde::{Deserialize, Serialize};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

#[derive(Debug, Clone, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: i64,
    messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    usage: Usage,
}

#[derive(Debug, Clone, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Usage {
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Clone)]
pub struct AnthropicLlmResult {
    pub content: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

pub async fn call_anthropic(
    api_key: &str,
    model: &str,
    prompt: &str,
) -> anyhow::Result<AnthropicLlmResult> {
    let client = reqwest::Client::new();
    let req_body = AnthropicRequest {
        model: model.to_string(),
        max_tokens: 1024,
        messages: vec![Message {
            role: "user".into(),
            content: prompt.into(),
        }],
    };

    let resp = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&req_body)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Anthropic API error {}: {}", status, body);
    }

    let json: AnthropicResponse = resp.json().await?;

    let content = json
        .content
        .into_iter()
        .find(|c| c.block_type == "text")
        .map(|c| c.text)
        .unwrap_or_default();

    Ok(AnthropicLlmResult {
        content,
        input_tokens: json.usage.input_tokens,
        output_tokens: json.usage.output_tokens,
    })
}
