use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::LoginResponseMsg;
use crate::infrastructure::http_request::{HttpClient, HttpResponseError};

#[derive(Serialize)]
struct Credentials<'a> {
    login_name: &'a str,
    password: &'a str,
}

#[derive(Serialize)]
struct RegistrationCredentials<'a> {
    login_name: &'a str,
    password: &'a str,
}

#[derive(Serialize)]
struct RefreshTokenRequest<'a> {
    refresh_token: &'a str,
}

pub async fn login(
    address: String,
    login_name: String,
    password: String,
) -> anyhow::Result<LoginResponseMsg> {
    request_session(
        format!("{address}/api/v1/auth/login"),
        Credentials {
            login_name: &login_name,
            password: &password,
        },
    )
    .await
    .map_err(map_login_error)
}

pub async fn register(
    address: String,
    login_name: String,
    password: String,
) -> anyhow::Result<LoginResponseMsg> {
    request_session(
        format!("{address}/api/v1/auth/register"),
        RegistrationCredentials {
            login_name: &login_name,
            password: &password,
        },
    )
    .await
    .map_err(map_register_error)
}

pub async fn refresh_token(
    address: String,
    refresh_token: String,
) -> anyhow::Result<LoginResponseMsg> {
    request_session(
        format!("{address}/api/v1/auth/refresh"),
        RefreshTokenRequest {
            refresh_token: &refresh_token,
        },
    )
    .await
}

pub async fn validate_token(
    address: String,
    user_id: String,
    access_token: String,
) -> anyhow::Result<LoginResponseMsg> {
    if access_token.trim().is_empty() {
        return Err(anyhow::anyhow!("登录状态缺少 access token"));
    }

    let mut url = reqwest::Url::parse(&format!("{address}/api/v1/user/get"))?;
    url.query_pairs_mut().append_pair("user_id", &user_id);

    let response = HttpClient::unauthenticated()
        .get_with_bearer(url.to_string(), &access_token)
        .await?;
    if !response.code.is_empty() && !response.code.starts_with('2') {
        return Err(anyhow::anyhow!(response.msg));
    }

    let user: ServerUser = serde_json::from_value(response.data)?;
    Ok(LoginResponseMsg {
        user_id: user.id,
        username: user.username,
        login_name: user.login_name,
        user_avatar: user.avatar,
        user_token: access_token,
        refresh_token: String::new(),
        access_token_expires_at: None,
        refresh_token_expires_at: None,
    })
}

async fn request_session<T: Serialize>(
    url: String,
    payload: T,
) -> anyhow::Result<LoginResponseMsg> {
    let response = HttpClient::unauthenticated()
        .post(url, serde_json::to_value(payload)?)
        .await?;

    if !response.code.is_empty() && !response.code.starts_with('2') {
        return Err(anyhow::anyhow!(response.msg));
    }

    decode_session(response.data)
}

fn map_login_error(error: anyhow::Error) -> anyhow::Error {
    map_auth_error(error, "用户不存在或密码错误", "请求参数无效")
}

fn map_register_error(error: anyhow::Error) -> anyhow::Error {
    map_auth_error(error, "注册接口未授权，请重启服务端", "登录名已存在")
}

fn map_auth_error(
    error: anyhow::Error,
    unauthorized_message: &'static str,
    conflict_message: &'static str,
) -> anyhow::Error {
    let status = error
        .downcast_ref::<HttpResponseError>()
        .map(HttpResponseError::status);

    match status {
        Some(reqwest::StatusCode::UNAUTHORIZED) => anyhow::Error::msg(unauthorized_message),
        Some(reqwest::StatusCode::CONFLICT) => anyhow::Error::msg(conflict_message),
        Some(reqwest::StatusCode::BAD_REQUEST) => anyhow::anyhow!("登录名和密码不能为空"),
        _ => error,
    }
}

#[derive(Deserialize)]
struct ServerLoginResult {
    user: ServerUser,
    #[serde(default, alias = "token")]
    access_token: String,
    #[serde(default)]
    refresh_token: String,
    #[serde(default)]
    access_token_expires_at: Option<i64>,
    #[serde(default)]
    refresh_token_expires_at: Option<i64>,
}

#[derive(Deserialize)]
struct ServerUser {
    id: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    login_name: String,
    avatar: String,
}

pub fn decode_session(data: Value) -> anyhow::Result<LoginResponseMsg> {
    if data.get("user").is_some() {
        let result: ServerLoginResult = serde_json::from_value(data)?;
        if result.access_token.is_empty() {
            return Err(anyhow::anyhow!("登录响应缺少 access token"));
        }
        return Ok(LoginResponseMsg {
            user_id: result.user.id,
            username: result.user.username,
            login_name: result.user.login_name,
            user_avatar: result.user.avatar,
            user_token: result.access_token,
            refresh_token: result.refresh_token,
            access_token_expires_at: result.access_token_expires_at,
            refresh_token_expires_at: result.refresh_token_expires_at,
        });
    }

    Ok(serde_json::from_value(data)?)
}
