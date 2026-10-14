use chrono::Utc;
use reqwest::Client;
use std::time::Duration;

pub async fn fetch_remote_content(url: &str) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("QuickHosts/0.1.0 (Rust Desktop Client)")
        .build()
        .map_err(|e| format!("Client creation error: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP Error Code: {}", response.status()));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    Ok(text)
}

pub fn should_update_remote(
    last_updated: Option<chrono::DateTime<Utc>>,
    interval: &crate::model::UpdateInterval,
) -> bool {
    let minutes = interval.to_minutes();
    if minutes == 0 {
        return false;
    }
    match last_updated {
        None => true,
        Some(last) => {
            let elapsed = Utc::now().signed_duration_since(last);
            elapsed.num_minutes() >= minutes as i64
        }
    }
}
