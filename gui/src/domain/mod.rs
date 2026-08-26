use serde::{Deserialize, Serialize};

pub mod search_entity;

fn default_allow_join() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct User {
    pub id: String,
    pub name: String,
    pub avatar: String,
    // pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GroupHistory {
    pub group_id: String,
    pub message_id: String,
    pub send_group_id: String,
    pub send_user_id: String,
    pub send_username: String,
    pub send_user_avatar: String,
    pub message: String,
    pub time: String,
    pub files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GroupMembers {
    pub group_id: String,
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub user_type: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MessageGroup {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub history: Vec<GroupHistory>,
    pub members: Vec<GroupMembers>,
    #[serde(rename = "type")]
    pub group_type: String,
    #[serde(default = "default_allow_join")]
    pub allow_join: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct WsMsgEvent {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AppNotification {
    pub id: String,
    #[serde(rename = "type")]
    pub notification_type: String,
    pub status: String,
    pub recipient_id: String,
    #[serde(default)]
    pub recipient_name: String,
    #[serde(default)]
    pub recipient_avatar: String,
    pub sender_id: String,
    #[serde(default)]
    pub sender_name: String,
    #[serde(default)]
    pub sender_avatar: String,
    pub group_id: String,
    #[serde(default)]
    pub group_name: String,
    #[serde(default)]
    pub group_avatar: String,
    pub handled_by_id: String,
    pub created_at: i64,
    pub handled_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UserDetailInfo {
    pub friends: Vec<User>,
    pub message_groups: Vec<MessageGroup>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LoginResponseMsg {
    #[serde(rename = "user_id")]
    pub user_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(rename = "login_name", default)]
    pub login_name: String,
    #[serde(rename = "user_avatar")]
    pub user_avatar: String,
    #[serde(rename = "user_token")]
    pub user_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub access_token_expires_at: Option<i64>,
    #[serde(default)]
    pub refresh_token_expires_at: Option<i64>,
}

impl LoginResponseMsg {
    const TOKEN_REFRESH_LEEWAY_SECONDS: i64 = 120;

    pub fn is_authenticated(&self) -> bool {
        !self.user_id.is_empty() && !self.user_token.is_empty()
    }

    pub fn needs_token_refresh(&self, now_timestamp: i64) -> bool {
        self.access_token_expires_at.is_some_and(|expires_at| {
            expires_at <= now_timestamp + Self::TOKEN_REFRESH_LEEWAY_SECONDS
        })
    }

    pub fn can_refresh_token(&self, now_timestamp: i64) -> bool {
        !self.refresh_token.is_empty()
            && self
                .refresh_token_expires_at
                .is_none_or(|expires_at| expires_at > now_timestamp)
    }

    pub fn merge_refresh_result(&self, mut refreshed: Self) -> Self {
        if refreshed.user_id.is_empty() {
            refreshed.user_id = self.user_id.clone();
        }
        if refreshed.username.is_empty() {
            refreshed.username = self.username.clone();
        }
        if refreshed.login_name.is_empty() {
            refreshed.login_name = self.login_name.clone();
        }
        if refreshed.user_avatar.is_empty() {
            refreshed.user_avatar = self.user_avatar.clone();
        }
        if refreshed.user_token.is_empty() {
            refreshed.user_token = self.user_token.clone();
        }
        if refreshed.refresh_token.is_empty() {
            refreshed.refresh_token = self.refresh_token.clone();
        }
        if refreshed.access_token_expires_at.is_none() {
            refreshed.access_token_expires_at = self.access_token_expires_at;
        }
        if refreshed.refresh_token_expires_at.is_none() {
            refreshed.refresh_token_expires_at = self.refresh_token_expires_at;
        }
        refreshed
    }
}
