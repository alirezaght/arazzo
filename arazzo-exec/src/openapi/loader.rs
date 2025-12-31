pub(crate) async fn load_openapi(
    client: &reqwest::Client,
    url_or_path: &str,
) -> Result<serde_json::Value, String> {
    if url_or_path.starts_with("http://") || url_or_path.starts_with("https://") {
        let resp = client
            .get(url_or_path)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("HTTP {status}"));
        }
        let body = resp.text().await.map_err(|e| e.to_string())?;
        parse_openapi_str(&body)
    } else {
        let body =
            std::fs::read_to_string(url_or_path).map_err(|e| format!("read file: {e}"))?;
        parse_openapi_str(&body)
    }
}

pub(crate) fn parse_openapi_str(body: &str) -> Result<serde_json::Value, String> {
    let trimmed = body.trim_start();
    if trimmed.starts_with('{') {
        serde_json::from_str::<serde_json::Value>(body).map_err(|e| e.to_string())
    } else {
        let y = serde_yaml::from_str::<serde_yaml::Value>(body).map_err(|e| e.to_string())?;
        serde_json::to_value(y).map_err(|e| e.to_string())
    }
}

