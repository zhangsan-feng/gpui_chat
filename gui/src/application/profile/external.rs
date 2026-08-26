use crate::domain::LoginResponseMsg;

use super::core;

pub async fn update_profile(
    address: String,
    username: String,
    login_name: String,
    current_password: String,
    new_password: String,
    avatar_path: Option<String>,
) -> anyhow::Result<LoginResponseMsg> {
    core::update_profile(
        address,
        username,
        login_name,
        current_password,
        new_password,
        avatar_path,
    )
    .await
}
