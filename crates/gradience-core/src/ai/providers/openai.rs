use serde::{Deserialize, Serialize};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Debug, Clone, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiResponse {
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Clone, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Clone, Deserialize)]
struct Usage {
    prompt_tokens: i64,
    completion_tokens: i64,
}

#[derive(Debug, Clone)]
pub struct OpenAiLlmResult {
    pub content: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

pub async fn call_openai(
    api_key: &str,
    model: &str,
    prompt: &str,
) -> anyhow::Result<OpenAiLlmResult> {
    let client = reqwest::Client::new();
    let req_body = OpenAiRequest {
        model: model.to_string(),
        messages: vec![Message {
            role: "user".into(),
            content: prompt.into(),
        }],
    };

    let resp = client
        .post(OPENAI_API_URL)
        .header("authorization", format!("Bearer {api_key}"))
        .header("content-type", "application/json")
        .json(&req_body)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("OpenAI API error {}: {}", status, body);
    }

    let json: OpenAiResponse = resp.json().await?;

    let content = json
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default();

    Ok(OpenAiLlmResult {
        content,
        input_tokens: json.usage.prompt_tokens,
        output_tokens: json.usage.completion_tokens,
    })
}
