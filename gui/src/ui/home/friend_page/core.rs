use gpui::{AsyncApp, Context, Window};
use gpui_tokio::Tokio;
use log::{error, info};
use serde::Deserialize;

use super::FriendPage;
use crate::application::{group, social};
use crate::domain::{GroupHistory, GroupMembers, MessageGroup, User, WsMsgEvent};
use crate::state::{EventBus, GlobalState};
use crate::ui::component::dialog;

pub fn init_component_data(page: &mut FriendPage, friends: Vec<User>, groups: Vec<MessageGroup>) {
    info!(
        "loaded {} friends and {} conversations",
        friends.len(),
        groups.len()
    );
    page.friends = friends;
    page.groups = groups
        .into_iter()
        .filter(|group| group.group_type == "group")
        .collect();
}

pub fn update_component_data(page: &mut FriendPage, event: WsMsgEvent) {
    match event.msg_type.as_str() {
        "add_friend" => match serde_json::from_value::<FriendAddedPayload>(event.data) {
            Ok(data) if !page.friends.iter().any(|friend| friend.id == data.id) => {
                let user = User {
                    id: data.id,
                    name: if data.name.is_empty() {
                        data.username
                    } else {
                        data.name
                    },
                    avatar: data.avatar,
                };
                page.friends.push(user);
            }
            Ok(_) => {}
            Err(error) => info!("failed to parse friend event: {}", error),
        },
        "friend.removed" => match serde_json::from_value::<RemovedFriendPayload>(event.data) {
            Ok(data) => remove_friend(page, &data.id),
            Err(error) => info!("failed to parse friend removed event: {}", error),
        },
        "conversation.created" => match serde_json::from_value::<CreatedGroupPayload>(event.data) {
            Ok(data) => add_group(page, map_created_group(data)),
            Err(error) => error!("failed to parse created conversation event: {}", error),
        },
        "create_group_chat" | "user_join_group_chat" => {
            match serde_json::from_value::<MessageGroup>(event.data) {
                Ok(data) => add_group(page, data),
                Err(error) => error!("failed to parse group event: {}", error),
            }
        }
        "group.disbanded" => match serde_json::from_value::<GroupDisbandedPayload>(event.data) {
            Ok(data) => remove_group(page, &data.group_id),
            Err(error) => error!("failed to parse group disbanded event: {}", error),
        },
        "group.updated" => match serde_json::from_value::<GroupUpdatedPayload>(event.data) {
            Ok(data) => update_group(page, data),
            Err(error) => error!("failed to parse group updated event: {}", error),
        },
        _ => {}
    }
}

pub fn send_friend_message(page: &mut FriendPage, cx: &mut Context<FriendPage>) {
    let user_id = page.select_user_id.clone();
    if user_id.is_empty() {
        return;
    }

    let state = cx.global::<GlobalState>().0.clone();
    state.update(cx, |_, cx| cx.emit(EventBus::OpenPrivateChat(user_id)));
}

pub fn send_group_message(page: &mut FriendPage, cx: &mut Context<FriendPage>) {
    let group_id = page.select_group_id.clone();
    if group_id.is_empty() {
        return;
    }

    let state = cx.global::<GlobalState>().0.clone();
    state.update(cx, |_, cx| cx.emit(EventBus::OpenConversation(group_id)));
}

pub fn delete_friend(_page: &mut FriendPage, friend_id: String, cx: &mut Context<FriendPage>) {
    if friend_id.is_empty() {
        return;
    }

    let state = cx.global::<GlobalState>().0.clone();
    let snapshot = state.read(cx).clone();
    let address = snapshot.http_server;
    let user_id = snapshot.user_state.user_id;
    let entity = cx.entity().clone();
    let mut async_cx = cx.to_async().clone();
    let task = Tokio::spawn(
        cx,
        social::remove_friend(address, user_id, friend_id.clone()),
    );

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        match task.await {
            Ok(Ok(())) => {
                let _ = entity.update(&mut async_cx, |page, cx| {
                    remove_friend(page, &friend_id);
                    cx.notify();
                });
            }
            Ok(Err(error)) => error!("failed to delete friend: {}", error),
            Err(error) => error!("delete friend task failed: {}", error),
        }
    })
    .detach();
}

pub fn leave_group(page: &mut FriendPage, group_id: String, cx: &mut Context<FriendPage>) {
    let Some(group) = page
        .groups
        .iter()
        .find(|group| group.id == group_id)
        .cloned()
    else {
        return;
    };
    let state = cx.global::<GlobalState>().0.clone();
    let snapshot = state.read(cx).clone();
    let address = snapshot.http_server;
    let user_id = snapshot.user_state.user_id;
    let is_owner = group
        .members
        .iter()
        .any(|member| member.id == user_id && member.user_type == "owner");
    let entity = cx.entity().clone();
    let mut async_cx = cx.to_async().clone();
    let group_id_for_task = group_id.clone();
    let task = Tokio::spawn(cx, async move {
        if is_owner {
            group::disband_group(address, group_id_for_task).await
        } else {
            group::leave_group(address, group_id_for_task).await
        }
    });

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        match task.await {
            Ok(Ok(())) => {
                let _ = entity.update(&mut async_cx, |page, cx| {
                    remove_group(page, &group_id);
                    let state = cx.global::<GlobalState>().0.clone();
                    state.update(cx, |_, cx| {
                        cx.emit(EventBus::RemoveConversation(group_id.clone()));
                    });
                    cx.notify();
                });
            }
            Ok(Err(error)) => error!("failed to leave group: {}", error),
            Err(error) => error!("leave group task failed: {}", error),
        }
    })
    .detach();
}

pub fn show_user_profile(page: &FriendPage, window: &mut Window, cx: &mut Context<FriendPage>) {
    let Some(user) = page
        .friends
        .iter()
        .find(|user| user.id == page.select_user_id)
        .cloned()
    else {
        return;
    };
    let description = format!(
        "昵称：{}\nID：{}\n头像：{}",
        user.name, user.id, user.avatar
    );
    dialog::info(window, cx, "好友资料", description);
}

pub fn show_group_info(page: &FriendPage, window: &mut Window, cx: &mut Context<FriendPage>) {
    let Some(group) = page
        .groups
        .iter()
        .find(|group| group.id == page.select_group_id)
        .cloned()
    else {
        return;
    };
    let description = format!(
        "群聊：{}\n群号：{}\n成员数：{}",
        group.name,
        group.id,
        group.members.len()
    );
    dialog::info(window, cx, "群聊资料", description);
}

pub fn remove_friend(page: &mut FriendPage, friend_id: &str) {
    page.friends.retain(|friend| friend.id != friend_id);
    if page.select_user_id == friend_id {
        page.select_user_id.clear();
    }
}

pub fn remove_group(page: &mut FriendPage, group_id: &str) {
    page.groups.retain(|group| group.id != group_id);
    if page.select_group_id == group_id {
        page.select_group_id.clear();
    }
}

fn add_group(page: &mut FriendPage, group: MessageGroup) {
    if group.group_type != "group" || page.groups.iter().any(|item| item.id == group.id) {
        return;
    }
    page.groups.push(group);
}

fn update_group(page: &mut FriendPage, data: GroupUpdatedPayload) {
    if let Some(group) = page
        .groups
        .iter_mut()
        .find(|group| group.id == data.group_id)
    {
        group.name = data.name;
        group.avatar = data.avatar;
        group.allow_join = data.allow_join;
    }
}

fn map_created_group(data: CreatedGroupPayload) -> MessageGroup {
    let group_id = data.id.clone();
    let history = data
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

    MessageGroup {
        id: group_id.clone(),
        name: data.name,
        avatar: data.avatar,
        history,
        members: data
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
            .collect(),
        group_type: if data.kind == "direct" {
            "private_chat".to_string()
        } else {
            data.kind
        },
        allow_join: data.allow_join,
    }
}

#[derive(Deserialize)]
struct CreatedGroupPayload {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    avatar: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    members: Vec<CreatedMemberPayload>,
    #[serde(default)]
    messages: Vec<CreatedMessagePayload>,
    #[serde(default = "default_allow_join")]
    allow_join: bool,
}

fn default_allow_join() -> bool {
    true
}

#[derive(Deserialize)]
struct GroupUpdatedPayload {
    group_id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    avatar: String,
    #[serde(default = "default_allow_join")]
    allow_join: bool,
}

#[derive(Deserialize)]
struct CreatedMemberPayload {
    #[serde(default)]
    user_id: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    avatar: String,
    #[serde(default)]
    role: String,
}

#[derive(Deserialize)]
struct CreatedMessagePayload {
    id: String,
    #[serde(default)]
    sender_id: String,
    #[serde(default)]
    sender_name: String,
    #[serde(default)]
    sender_avatar: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    attachments: Vec<CreatedAttachmentPayload>,
    created_at: i64,
}

#[derive(Deserialize)]
struct CreatedAttachmentPayload {
    #[serde(default)]
    url: String,
}

#[derive(Deserialize)]
struct GroupDisbandedPayload {
    group_id: String,
}

#[derive(Deserialize)]
struct RemovedFriendPayload {
    id: String,
}

#[derive(Deserialize)]
struct FriendAddedPayload {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    avatar: String,
}
