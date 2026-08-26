use serde_json::json;

use crate::domain::AppNotification;
use crate::infrastructure::http_request::HttpClient;

pub async fn list(address: String) -> anyhow::Result<Vec<AppNotification>> {
    let response = HttpClient::new()
        .get(format!("{address}/api/v1/notification/list"))
        .await?;
    Ok(serde_json::from_value(response.data)?)
}

pub async fn review_group_join_request(
    address: String,
    notification_id: String,
    approved: bool,
) -> anyhow::Result<AppNotification> {
    let response = HttpClient::new()
        .post(
            format!("{address}/api/v1/group/review-join-request"),
            json!({
                "notification_id": notification_id,
                "approved": approved,
            }),
        )
        .await?;
    Ok(serde_json::from_value(response.data)?)
}

pub async fn review_friend_request(
    address: String,
    notification_id: String,
    approved: bool,
) -> anyhow::Result<AppNotification> {
    let response = HttpClient::new()
        .post(
            format!("{address}/api/v1/friend/review-request"),
            json!({
                "notification_id": notification_id,
                "approved": approved,
            }),
        )
        .await?;
    Ok(serde_json::from_value(response.data)?)
}
