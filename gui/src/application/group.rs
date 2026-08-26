use std::path::PathBuf;

use reqwest::multipart;
use serde_json::json;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::infrastructure::http_request::HttpClient;

pub async fn create_private_chat(
    _address: String,
    _current_user_id: String,
    _target_user_id: String,
) -> anyhow::Result<()> {
    Ok(())
}

pub async fn create_group_chat(
    address: String,
    _current_user_id: String,
    group_name: String,
    avatar_path: Option<PathBuf>,
) -> anyhow::Result<bool> {
    let mut form = multipart::Form::new().text("name", group_name);

    if let Some(path) = avatar_path {
        let file = File::open(&path).await?;
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = reqwest::Body::wrap_stream(stream);
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "avatar".to_string());
        form = form.part("avatar", multipart::Part::stream(body).file_name(file_name));
    }

    HttpClient::new()
        .post_form(format!("{address}/api/v1/group/create"), form)
        .await?;
    Ok(true)
}

pub async fn leave_group(address: String, group_id: String) -> anyhow::Result<()> {
    HttpClient::new()
        .post(
            format!("{address}/api/v1/group/leave"),
            json!({"group_id": group_id}),
        )
        .await?;
    Ok(())
}

pub async fn disband_group(address: String, group_id: String) -> anyhow::Result<()> {
    HttpClient::new()
        .post(
            format!("{address}/api/v1/group/disband"),
            json!({"group_id": group_id}),
        )
        .await?;
    Ok(())
}

pub async fn update_member_role(
    address: String,
    group_id: String,
    user_id: String,
    role: String,
) -> anyhow::Result<()> {
    HttpClient::new()
        .post(
            format!("{address}/api/v1/group/member-role"),
            json!({
                "group_id": group_id,
                "user_id": user_id,
                "role": role,
            }),
        )
        .await?;
    Ok(())
}

pub async fn update_group(
    address: String,
    group_id: String,
    group_name: String,
    avatar_path: Option<PathBuf>,
    allow_join: bool,
) -> anyhow::Result<()> {
    let mut form = multipart::Form::new()
        .text("group_id", group_id)
        .text("name", group_name)
        .text("allow_join", allow_join.to_string());

    if let Some(path) = avatar_path {
        let file = File::open(&path).await?;
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = reqwest::Body::wrap_stream(stream);
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "avatar".to_string());
        form = form.part("avatar", multipart::Part::stream(body).file_name(file_name));
    }

    HttpClient::new()
        .post_form(format!("{address}/api/v1/group/update"), form)
        .await?;
    Ok(())
}
