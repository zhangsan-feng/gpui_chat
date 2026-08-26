use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use futures_util::TryStreamExt;
use log::info;
use reqwest::header::CONTENT_TYPE;
use reqwest::{RequestBuilder, StatusCode, multipart};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RestResponse<T> {
    #[serde(rename = "code")]
    #[serde(default)]
    pub code: String,
    #[serde(rename = "data")]
    pub data: T,
    #[serde(rename = "msg")]
    #[serde(default)]
    pub msg: String,
}

#[derive(Debug)]
pub struct HttpResponseError {
    status: StatusCode,
    message: String,
}

impl HttpResponseError {
    pub fn status(&self) -> StatusCode {
        self.status
    }
}

impl Display for HttpResponseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for HttpResponseError {}

trait ResponseHandler {
    async fn handle(self) -> Result<RestResponse<Value>, anyhow::Error>;
}

impl ResponseHandler for reqwest::Response {
    async fn handle(self) -> Result<RestResponse<Value>, anyhow::Error> {
        let status = self.status();
        let bytes = self.bytes().await.unwrap_or_default();
        let body_str = String::from_utf8_lossy(&bytes);

        if status.is_success() {
            match serde_json::from_slice(&bytes) {
                Ok(data) => Ok(data),
                Err(error) => {
                    info!("序列化失败: {}, 响应内容: {}", error, body_str);
                    Err(anyhow::anyhow!(
                        "序列化失败: {}, 响应内容: {}",
                        error,
                        body_str
                    ))
                }
            }
        } else {
            info!("请求失败, 状态码: {}, 响应: {}", status, body_str);
            let message = response_error_message(&bytes)
                .unwrap_or_else(|| format!("请求失败, 状态码: {status}, 响应: {body_str}"));
            Err(HttpResponseError { status, message }.into())
        }
    }
}

fn response_error_message(bytes: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    ["error", "message", "msg"]
        .into_iter()
        .find_map(|field| value.get(field).and_then(Value::as_str))
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToOwned::to_owned)
}

#[derive(Clone, Default)]
struct AuthTokens {
    access_token: String,
    refresh_token: String,
}

static AUTH_TOKENS: OnceLock<Mutex<AuthTokens>> = OnceLock::new();
static AUTH_EXPIRED: AtomicBool = AtomicBool::new(false);

pub fn set_auth_tokens(access_token: &str, refresh_token: &str) {
    let tokens = AUTH_TOKENS.get_or_init(|| Mutex::new(AuthTokens::default()));
    if let Ok(mut current) = tokens.lock() {
        current.access_token = access_token.to_string();
        current.refresh_token = refresh_token.to_string();
    }
    AUTH_EXPIRED.store(false, Ordering::Release);
}

pub fn clear_auth_tokens() {
    let tokens = AUTH_TOKENS.get_or_init(|| Mutex::new(AuthTokens::default()));
    if let Ok(mut current) = tokens.lock() {
        *current = AuthTokens::default();
    }
}

pub fn take_auth_expired() -> bool {
    AUTH_EXPIRED.swap(false, Ordering::AcqRel)
}

fn auth_tokens() -> AuthTokens {
    AUTH_TOKENS
        .get_or_init(|| Mutex::new(AuthTokens::default()))
        .lock()
        .map(|tokens| tokens.clone())
        .unwrap_or_default()
}

pub struct HttpClient {
    client: reqwest::Client,
    authenticated: bool,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            authenticated: true,
        }
    }

    pub fn unauthenticated() -> Self {
        Self {
            client: reqwest::Client::new(),
            authenticated: false,
        }
    }

    pub async fn get(&self, url: String) -> Result<RestResponse<Value>, anyhow::Error> {
        self.execute(&url, |access_token| {
            let mut request = self.client.get(&url);
            if let Some(access_token) = access_token {
                request = request.bearer_auth(access_token);
            }
            request
        })
        .await
    }

    pub async fn get_with_bearer(
        &self,
        url: String,
        access_token: &str,
    ) -> Result<RestResponse<Value>, anyhow::Error> {
        let response = self
            .send(&url, self.client.get(&url).bearer_auth(access_token))
            .await?;
        response.handle().await
    }

    pub async fn post(
        &self,
        url: String,
        body: Value,
    ) -> Result<RestResponse<Value>, anyhow::Error> {
        self.execute(&url, |access_token| {
            let mut request = self.client.post(&url).json(&body);
            if let Some(access_token) = access_token {
                request = request.bearer_auth(access_token);
            }
            request
        })
        .await
    }

    pub async fn post_form(
        &self,
        url: String,
        form: multipart::Form,
    ) -> Result<RestResponse<Value>, anyhow::Error> {
        let content_type = format!("multipart/form-data; boundary={}", form.boundary());
        let body = form
            .into_stream()
            .try_fold(Vec::new(), |mut body, chunk| async move {
                body.extend_from_slice(&chunk);
                Ok(body)
            })
            .await
            .map_err(|error| anyhow::anyhow!("读取 multipart 请求体失败: {error}"))?;

        self.execute(&url, |access_token| {
            let mut request = self
                .client
                .post(&url)
                .header(CONTENT_TYPE, &content_type)
                .body(body.clone());
            if let Some(access_token) = access_token {
                request = request.bearer_auth(access_token);
            }
            request
        })
        .await
    }

    async fn execute<F>(
        &self,
        url: &str,
        build_request: F,
    ) -> Result<RestResponse<Value>, anyhow::Error>
    where
        F: Fn(Option<&str>) -> RequestBuilder,
    {
        let access_token = self.authenticated.then(|| auth_tokens().access_token);
        let response = self
            .send(url, build_request(access_token.as_deref()))
            .await?;

        if self.authenticated && response.status() == StatusCode::UNAUTHORIZED {
            match self.refresh_tokens(url).await {
                Ok(Some(refreshed)) => {
                    let response = self
                        .send(url, build_request(Some(&refreshed.access_token)))
                        .await?;
                    if response.status() == StatusCode::UNAUTHORIZED {
                        mark_auth_expired();
                    }
                    return response.handle().await;
                }
                Ok(None) => mark_auth_expired(),
                Err(error) => {
                    info!("自动刷新 token 失败: {}", error);
                    mark_auth_expired();
                }
            }
        }

        response.handle().await
    }

    async fn send(
        &self,
        url: &str,
        request: RequestBuilder,
    ) -> Result<reqwest::Response, anyhow::Error> {
        request
            .send()
            .await
            .map_err(|error| anyhow::anyhow!("请求失败 [{url}]: {error}"))
    }

    async fn refresh_tokens(&self, request_url: &str) -> anyhow::Result<Option<AuthTokens>> {
        let current = auth_tokens();
        if current.refresh_token.is_empty() {
            return Ok(None);
        }

        let response = self
            .client
            .post(refresh_url(request_url)?)
            .json(&json!({ "refresh_token": current.refresh_token }))
            .send()
            .await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            return Ok(None);
        }

        let response = response.handle().await?;
        let refreshed = parse_auth_tokens(response.data, current.refresh_token)?;
        set_auth_tokens(&refreshed.access_token, &refreshed.refresh_token);
        Ok(Some(refreshed))
    }
}

fn mark_auth_expired() {
    clear_auth_tokens();
    AUTH_EXPIRED.store(true, Ordering::Release);
}

fn refresh_url(request_url: &str) -> anyhow::Result<String> {
    let mut url = reqwest::Url::parse(request_url)?;
    url.set_path("/api/v1/auth/refresh");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

#[derive(Deserialize)]
struct TokenData {
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    user_token: String,
    #[serde(default)]
    token: String,
    #[serde(default)]
    refresh_token: String,
}

fn parse_auth_tokens(data: Value, previous_refresh_token: String) -> anyhow::Result<AuthTokens> {
    let token_data: TokenData = serde_json::from_value(data)?;
    let access_token = if token_data.access_token.is_empty() {
        if token_data.user_token.is_empty() {
            token_data.token
        } else {
            token_data.user_token
        }
    } else {
        token_data.access_token
    };

    if access_token.is_empty() {
        return Err(anyhow::anyhow!("刷新 token 响应缺少 access token"));
    }

    Ok(AuthTokens {
        access_token,
        refresh_token: if token_data.refresh_token.is_empty() {
            previous_refresh_token
        } else {
            token_data.refresh_token
        },
    })
}
