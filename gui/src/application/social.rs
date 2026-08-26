use serde::Deserialize;
use serde_json::json;

use crate::domain::search_entity::{SearchGroupResult, SearchResult, SearchUserResult};
use crate::infrastructure::http_request::HttpClient;

pub async fn search(
    address: String,
    _user_id: String,
    keyword: String,
) -> anyhow::Result<SearchResult> {
    let mut url = reqwest::Url::parse(&format!("{address}/api/v1/search"))?;
    url.query_pairs_mut().append_pair("keyword", &keyword);
    let response = HttpClient::new().get(url.to_string()).await?;
    let result: ServerSearchResult = serde_json::from_value(response.data)?;

    Ok(SearchResult {
        groups: result
            .groups
            .into_iter()
            .map(|group| SearchGroupResult {
                id: group.id,
                name: group.name,
                avatar: group.avatar,
            })
            .collect(),
        users: result
            .users
            .into_iter()
            .map(|user| SearchUserResult {
                id: user.id,
                name: user.username,
                avatar: user.avatar,
            })
            .collect(),
    })
}

pub async fn join_group(address: String, _user_id: String, group_id: String) -> anyhow::Result<()> {
    HttpClient::new()
        .post(
            format!("{address}/api/v1/group/request-join"),
            json!({"group_id": group_id}),
        )
        .await?;
    Ok(())
}

pub async fn add_friend(
    address: String,
    _self_user_id: String,
    friend_user_id: String,
) -> anyhow::Result<()> {
    HttpClient::new()
        .post(
            format!("{address}/api/v1/friend/request"),
            json!({"friend_id": friend_user_id}),
        )
        .await?;
    Ok(())
}

pub async fn remove_friend(
    address: String,
    _self_user_id: String,
    friend_user_id: String,
) -> anyhow::Result<()> {
    HttpClient::new()
        .post(
            format!("{address}/api/v1/friend/remove"),
            json!({"friend_id": friend_user_id}),
        )
        .await?;
    Ok(())
}

#[derive(Deserialize)]
struct ServerSearchResult {
    #[serde(default)]
    groups: Vec<ServerGroupResult>,
    #[serde(default)]
    users: Vec<ServerUserResult>,
}

#[derive(Deserialize)]
struct ServerGroupResult {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    avatar: String,
}

#[derive(Deserialize)]
struct ServerUserResult {
    id: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    avatar: String,
}
