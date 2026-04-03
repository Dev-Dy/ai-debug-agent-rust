// services/ai_service.rs
use reqwest;

pub async fn call_ai(logs: String) -> String {
    let client = reqwest::Client::new();

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", "Bearer YOUR_API_KEY")
        .json(&serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [
                {"role": "system", "content": "You are a backend debugging expert."},
                {"role": "user", "content": logs}
            ]
        }))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = response.json().await.unwrap();

    body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No response")
        .to_string()
}