use crate::domain::LoginResponseMsg;

use super::core;

pub async fn login(
    address: String,
    login_name: String,
    password: String,
) -> anyhow::Result<LoginResponseMsg> {
    core::login(address, login_name, password).await
}

pub async fn register(
    address: String,
    login_name: String,
    password: String,
) -> anyhow::Result<LoginResponseMsg> {
    core::register(address, login_name, password).await
}

pub fn decode_session(data: serde_json::Value) -> anyhow::Result<LoginResponseMsg> {
    core::decode_session(data)
}

pub async fn refresh_token(
    address: String,
    refresh_token: String,
) -> anyhow::Result<LoginResponseMsg> {
    core::refresh_token(address, refresh_token).await
}

pub async fn validate_token(
    address: String,
    user_id: String,
    access_token: String,
) -> anyhow::Result<LoginResponseMsg> {
    core::validate_token(address, user_id, access_token).await
}
