use std::path::PathBuf;

use reqwest::multipart;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::application::auth;
use crate::infrastructure::http_request::{HttpClient, HttpResponseError};

pub async fn update_profile(
    address: String,
    username: String,
    login_name: String,
    current_password: String,
    new_password: String,
    avatar_path: Option<String>,
) -> anyhow::Result<crate::domain::LoginResponseMsg> {
    let mut form = multipart::Form::new()
        .text("username", username)
        .text("login_name", login_name)
        .text("current_password", current_password)
        .text("new_password", new_password);

    if let Some(path) = avatar_path.map(PathBuf::from) {
        let file = File::open(&path).await?;
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = reqwest::Body::wrap_stream(stream);
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "avatar".to_string());
        form = form.part("avatar", multipart::Part::stream(body).file_name(file_name));
    }

    let response = HttpClient::new()
        .post_form(format!("{address}/api/v1/user/profile"), form)
        .await
        .map_err(map_profile_error)?;
    auth::decode_session(response.data)
}

fn map_profile_error(error: anyhow::Error) -> anyhow::Error {
    let status = error
        .downcast_ref::<HttpResponseError>()
        .map(HttpResponseError::status);

    match status {
        Some(reqwest::StatusCode::FORBIDDEN) => anyhow::Error::msg("当前密码错误"),
        Some(reqwest::StatusCode::CONFLICT) => anyhow::Error::msg("登录名已存在"),
        _ => error,
    }
}
