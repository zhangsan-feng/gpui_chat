use serde::Deserialize;

use crate::domain::{GroupHistory, GroupMembers, MessageGroup, User, UserDetailInfo};
use crate::infrastructure::http_request::HttpClient;

pub async fn load_component_data(
    address: String,
    user_id: String,
) -> anyhow::Result<UserDetailInfo> {
    let client = HttpClient::new();
    let user_response = client
        .get(format!("{address}/api/v1/user/get?user_id={user_id}"))
        .await?;
    let conversation_response = client
        .get(format!("{address}/api/v1/conversation/list"))
        .await?;

    let user: ServerUser = serde_json::from_value(user_response.data)?;
    let groups: Vec<ServerGroup> = serde_json::from_value(conversation_response.data)?;

    Ok(UserDetailInfo {
        friends: user
            .friends
            .into_iter()
            .map(|friend| User {
                id: friend.id,
                name: friend.username,
                avatar: friend.avatar,
            })
            .collect(),
        message_groups: groups.into_iter().map(map_group).collect(),
    })
}

#[derive(Deserialize)]
struct ServerUser {
    #[serde(default)]
    friends: Vec<ServerFriend>,
}

#[derive(Deserialize)]
struct ServerFriend {
    id: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    avatar: String,
}

#[derive(Deserialize)]
struct ServerGroup {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    avatar: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    members: Vec<ServerMember>,
    #[serde(default)]
    messages: Vec<ServerMessage>,
    #[serde(default = "default_allow_join")]
    allow_join: bool,
}

fn default_allow_join() -> bool {
    true
}

#[derive(Deserialize)]
struct ServerMember {
    user_id: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    avatar: String,
    #[serde(default)]
    role: String,
}

#[derive(Deserialize)]
struct ServerMessage {
    id: String,
    sender_id: String,
    #[serde(default)]
    sender_name: String,
    #[serde(default)]
    sender_avatar: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    attachments: Vec<ServerAttachment>,
    created_at: i64,
}

#[derive(Deserialize)]
struct ServerAttachment {
    #[serde(default)]
    url: String,
}

fn map_group(group: ServerGroup) -> MessageGroup {
    let group_id = group.id.clone();
    let members = group
        .members
        .into_iter()
        .map(|member| GroupMembers {
            group_id: group_id.clone(),
            id: member.user_id,
            name: member.username,
            avatar: member.avatar,
            user_type: member.role,
            status: String::new(),
        })
        .collect();
    let history = group
        .messages
        .into_iter()
        .map(|message| GroupHistory {
            group_id: group_id.clone(),
            message_id: message.id,
            send_group_id: group_id.clone(),
            send_user_id: message.sender_id,
            send_username: message.sender_name,
            send_user_avatar: message.sender_avatar,
            message: message.content,
            time: message.created_at.to_string(),
            files: message
                .attachments
                .into_iter()
                .map(|attachment| attachment.url)
                .collect(),
        })
        .collect();

    let group_type = if group.kind == "direct" {
        "private_chat".to_string()
    } else {
        group.kind
    };

    MessageGroup {
        id: group_id,
        name: group.name,
        avatar: group.avatar,
        history,
        members,
        group_type,
        allow_join: group.allow_join,
    }
}
