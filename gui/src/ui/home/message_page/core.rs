use std::collections::HashMap;

use gpui::{AppContext, Context, DragMoveEvent, Window};
use gpui_component::VirtualListScrollHandle;
use gpui_component::input::InputState;
use log::{error, info};
use serde::Deserialize;

use super::group_settings::GroupSettingsPanel;
use super::group_user::GroupMemberEntity;
use super::message::HistoryMessageEntity;
use super::send_message::SendMessageEntity;
use super::{HistoryMessagePanelResizeHandle, LeftPanelResizeHandle, MessagePage};
use crate::domain::{GroupHistory, GroupMembers, MessageGroup, WsMsgEvent};

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
    #[serde(default)]
    group_id: String,
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
struct GroupMemberLeftPayload {
    group_id: String,
    user_id: String,
}

#[derive(Deserialize)]
struct GroupDisbandedPayload {
    group_id: String,
}

#[derive(Deserialize)]
struct ConversationDeletedPayload {
    group_id: String,
}

#[derive(Deserialize)]
struct GroupMemberRoleChangedPayload {
    group_id: String,
    user_id: String,
    role: String,
}

#[derive(Deserialize)]
struct GroupMemberAddedPayload {
    group_id: String,
    member: CreatedMemberPayload,
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

pub fn new(cx: &mut Context<MessagePage>, window: &mut Window) -> MessagePage {
    MessagePage {
        select_index: 0,
        message_group: Vec::new(),
        search_input: cx.new(|cx| InputState::new(window, cx).placeholder("搜索")),
        message_group_scroll_handle: VirtualListScrollHandle::new(),
        unread_counts: HashMap::new(),
        context_menu_group_id: None,
        left_panel_default_width: 330.0,
        left_panel_min_width: 200.0,
        left_panel_max_width: 330.0,
        history_message_panel_min_height: 140.0,
        history_message_panel_max_height: 500.0,
        sned_message_entity: cx.new(|cx| SendMessageEntity::new(window, cx)),
        history_message_entity: cx.new(|cx| HistoryMessageEntity::new(window, cx)),
        group_members_entity: cx.new(|cx| GroupMemberEntity::new(window, cx)),
        group_settings_entity: cx.new(|cx| GroupSettingsPanel::new(window, cx)),
    }
}

pub fn update_component_data(
    page: &mut MessagePage,
    event: WsMsgEvent,
    cx: &mut Context<MessagePage>,
) {
    match event.msg_type.as_str() {
        "message" => {
            if let Ok(data) = serde_json::from_value::<GroupHistory>(event.data) {
                append_message(page, data, cx);
            }
        }
        "message.created" => match serde_json::from_value::<CreatedMessagePayload>(event.data) {
            Ok(data) => append_message(page, map_created_message(data), cx),
            Err(error) => error!("failed to parse created message event: {}", error),
        },
        "conversation.created" => match serde_json::from_value::<CreatedGroupPayload>(event.data) {
            Ok(data) => insert_created_group(page, map_created_group(data), cx),
            Err(error) => error!("failed to parse created conversation event: {}", error),
        },
        "conversation.deleted" => {
            match serde_json::from_value::<ConversationDeletedPayload>(event.data) {
                Ok(data) => remove_group(page, &data.group_id, cx),
                Err(error) => error!("failed to parse deleted conversation event: {}", error),
            }
        }
        "create_group_chat" => {
            if let Ok(data) = serde_json::from_value::<MessageGroup>(event.data) {
                insert_created_group(page, data, cx);
            }
        }
        "other_join_group_chat" => {
            if let Ok(data) = serde_json::from_value::<GroupMembers>(event.data) {
                if let Some(group) = page
                    .message_group
                    .iter_mut()
                    .find(|group| group.id == data.group_id)
                {
                    if group.members.iter().all(|member| member.id != data.id) {
                        group.members.push(data);
                        cx.notify();
                    }
                }
            }
        }
        "user_join_group_chat" => {
            if let Ok(data) = serde_json::from_value::<MessageGroup>(event.data) {
                insert_created_group(page, data, cx);
            }
        }
        "group.member_left" => match serde_json::from_value::<GroupMemberLeftPayload>(event.data) {
            Ok(data) => remove_group_member(page, data, cx),
            Err(error) => error!("failed to parse group member left event: {}", error),
        },
        "group.disbanded" => match serde_json::from_value::<GroupDisbandedPayload>(event.data) {
            Ok(data) => remove_group(page, &data.group_id, cx),
            Err(error) => error!("failed to parse group disbanded event: {}", error),
        },
        "group.member_added" => {
            match serde_json::from_value::<GroupMemberAddedPayload>(event.data) {
                Ok(data) => add_group_member(page, data, cx),
                Err(error) => error!("failed to parse group member added event: {}", error),
            }
        }
        "group.member_role_changed" => {
            match serde_json::from_value::<GroupMemberRoleChangedPayload>(event.data) {
                Ok(data) => update_group_member_role(page, data, cx),
                Err(error) => error!("failed to parse group member role event: {}", error),
            }
        }
        "group.updated" => match serde_json::from_value::<GroupUpdatedPayload>(event.data) {
            Ok(data) => update_group(page, data, cx),
            Err(error) => error!("failed to parse group updated event: {}", error),
        },
        message_type => info!("unknown message type: {}", message_type),
    }
}

pub fn init_component_data(
    page: &mut MessagePage,
    data: Vec<MessageGroup>,
    cx: &mut Context<MessagePage>,
) {
    page.unread_counts.clear();
    page.message_group = data;
    for group in &page.message_group {
        page.unread_counts
            .insert(group.id.clone(), group.history.len());
    }
    if let Some(first_group) = page.message_group.first().cloned() {
        select_group(page, &first_group, cx, false);
    }
}

pub fn open_conversation(page: &mut MessagePage, group_id: String, cx: &mut Context<MessagePage>) {
    let Some(index) = page
        .message_group
        .iter()
        .position(|group| group.id == group_id)
    else {
        return;
    };

    page.select_index = index;
    let group = page.message_group[index].clone();
    select_group(page, &group, cx, false);
    cx.notify();
}

pub fn open_private_chat(page: &mut MessagePage, user_id: String, cx: &mut Context<MessagePage>) {
    let Some(index) = page.message_group.iter().position(|group| {
        group.group_type == "private_chat"
            && group.members.iter().any(|member| member.id == user_id)
    }) else {
        info!("private chat not found for user: {}", user_id);
        return;
    };

    page.select_index = index;
    let group = page.message_group[index].clone();
    select_group(page, &group, cx, false);
    cx.notify();
}

pub fn left_panel_handle_resize(
    page: &mut MessagePage,
    event: &DragMoveEvent<LeftPanelResizeHandle>,
) {
    let new_width = event.event.position.x - gpui::px(60.0);
    page.left_panel_default_width = f32::from(new_width.clamp(
        gpui::px(page.left_panel_min_width),
        gpui::px(page.left_panel_max_width),
    ));
}

pub fn history_message_handle_resize(
    page: &mut MessagePage,
    event: &DragMoveEvent<HistoryMessagePanelResizeHandle>,
    window: &mut Window,
    cx: &mut Context<MessagePage>,
) {
    let new_height = window.bounds().size.height - event.event.position.y;
    page.sned_message_entity.update(cx, |sender, _| {
        sender.panel_height = f32::from(new_height.clamp(
            gpui::px(page.history_message_panel_min_height),
            gpui::px(page.history_message_panel_max_height),
        ));
    });
}

pub fn remove_group(page: &mut MessagePage, group_id: &str, cx: &mut Context<MessagePage>) {
    let Some(index) = page
        .message_group
        .iter()
        .position(|group| group.id == group_id)
    else {
        return;
    };

    let previous_selected_index = page.select_index;
    page.message_group.remove(index);
    page.unread_counts.remove(group_id);
    page.context_menu_group_id = None;

    if page.message_group.is_empty() {
        page.select_index = 0;
        page.sned_message_entity.update(cx, |sender, _| {
            sender.group_id.clear();
        });
        page.history_message_entity.update(cx, |history, cx| {
            history.history_message.clear();
            history.scroll_handle.reset(0);
            cx.notify();
        });
        page.group_members_entity.update(cx, |members, cx| {
            members.group_users.clear();
            members.group_type.clear();
            cx.notify();
        });
        cx.notify();
        return;
    }

    if index < previous_selected_index {
        page.select_index = previous_selected_index - 1;
    } else {
        page.select_index = previous_selected_index.min(page.message_group.len() - 1);
    }
    let selected_group = page.message_group[page.select_index].clone();
    select_group(page, &selected_group, cx, false);
    cx.notify();
}

fn remove_group_member(
    page: &mut MessagePage,
    data: GroupMemberLeftPayload,
    cx: &mut Context<MessagePage>,
) {
    let Some(updated_members) = page
        .message_group
        .iter_mut()
        .find(|group| group.id == data.group_id)
        .map(|group| {
            group.members.retain(|member| member.id != data.user_id);
            group.members.clone()
        })
    else {
        return;
    };

    if page
        .message_group
        .get(page.select_index)
        .is_some_and(|selected| selected.id == data.group_id)
    {
        page.group_members_entity.update(cx, |members, cx| {
            members.group_users = updated_members;
            cx.notify();
        });
    }
    cx.notify();
}

fn update_group_member_role(
    page: &mut MessagePage,
    data: GroupMemberRoleChangedPayload,
    cx: &mut Context<MessagePage>,
) {
    let Some(updated_members) = page
        .message_group
        .iter_mut()
        .find(|group| group.id == data.group_id)
        .map(|group| {
            if let Some(member) = group
                .members
                .iter_mut()
                .find(|member| member.id == data.user_id)
            {
                member.user_type = data.role.clone();
            }
            group.members.clone()
        })
    else {
        return;
    };

    if page
        .message_group
        .get(page.select_index)
        .is_some_and(|selected| selected.id == data.group_id)
    {
        page.group_members_entity.update(cx, |members, cx| {
            members.group_users = updated_members;
            cx.notify();
        });
    }
    cx.notify();
}

fn add_group_member(
    page: &mut MessagePage,
    data: GroupMemberAddedPayload,
    cx: &mut Context<MessagePage>,
) {
    let member = GroupMembers {
        group_id: data.group_id.clone(),
        id: data.member.user_id,
        name: data.member.username,
        avatar: data.member.avatar,
        user_type: data.member.role,
        status: String::new(),
    };
    let Some(updated_members) = page
        .message_group
        .iter_mut()
        .find(|group| group.id == data.group_id)
        .map(|group| {
            if group.members.iter().all(|current| current.id != member.id) {
                group.members.push(member);
            }
            group.members.clone()
        })
    else {
        return;
    };

    if page
        .message_group
        .get(page.select_index)
        .is_some_and(|selected| selected.id == data.group_id)
    {
        page.group_members_entity.update(cx, |members, cx| {
            members.group_users = updated_members;
            cx.notify();
        });
    }
    cx.notify();
}

fn map_created_group(data: CreatedGroupPayload) -> MessageGroup {
    let group_id = data.id.clone();
    let group_type = if data.kind == "direct" {
        "private_chat".to_string()
    } else if data.kind.is_empty() {
        "group".to_string()
    } else {
        data.kind
    };
    let history = data
        .messages
        .into_iter()
        .map(|mut message| {
            if message.group_id.is_empty() {
                message.group_id = group_id.clone();
            }
            map_created_message(message)
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
        group_type,
        allow_join: data.allow_join,
    }
}

fn update_group(page: &mut MessagePage, data: GroupUpdatedPayload, cx: &mut Context<MessagePage>) {
    let Some(group) = page
        .message_group
        .iter_mut()
        .find(|group| group.id == data.group_id)
    else {
        return;
    };
    group.name = data.name;
    group.avatar = data.avatar;
    group.allow_join = data.allow_join;
    cx.notify();
}

fn map_created_message(data: CreatedMessagePayload) -> GroupHistory {
    GroupHistory {
        group_id: data.group_id.clone(),
        message_id: data.id,
        send_group_id: data.group_id,
        send_user_id: data.sender_id,
        send_username: data.sender_name,
        send_user_avatar: data.sender_avatar,
        message: data.content,
        time: data.created_at.to_string(),
        files: data
            .attachments
            .into_iter()
            .map(|attachment| attachment.url)
            .collect(),
    }
}

fn append_message(page: &mut MessagePage, data: GroupHistory, cx: &mut Context<MessagePage>) {
    let group_id = data.group_id.clone();
    let message_id = data.message_id.clone();
    let Some(index) = page
        .message_group
        .iter()
        .position(|group| group.id == group_id)
    else {
        return;
    };

    if page.message_group[index]
        .history
        .iter()
        .any(|message| message.message_id == message_id)
    {
        return;
    }

    let selected_group = page.select_index == index;
    page.message_group[index].history.push(data);
    if !selected_group {
        *page.unread_counts.entry(group_id.clone()).or_default() += 1;
    }
    if index != 0 {
        let group = page.message_group.remove(index);
        page.message_group.insert(0, group);
        if selected_group {
            page.select_index = 0;
        } else if page.select_index < index {
            page.select_index += 1;
        }
    }

    if page
        .message_group
        .get(page.select_index)
        .is_some_and(|group| group.id == group_id)
    {
        let group = page.message_group[page.select_index].clone();
        page.history_message_entity.update(cx, |history, cx| {
            history.history_message = group.history;
            history.scroll_handle.reset(history.history_message.len());
            cx.notify();
        });
    }
    cx.notify();
}

fn insert_created_group(page: &mut MessagePage, data: MessageGroup, cx: &mut Context<MessagePage>) {
    if page.message_group.iter().any(|group| group.id == data.id) {
        return;
    }

    let group_id = data.id.clone();
    select_group(page, &data, cx, false);
    page.message_group.insert(0, data);
    page.select_index = 0;
    page.unread_counts.insert(group_id, 0);
    cx.notify();
}

fn select_group(
    page: &mut MessagePage,
    group: &MessageGroup,
    cx: &mut Context<MessagePage>,
    clear_history: bool,
) {
    page.unread_counts.insert(group.id.clone(), 0);
    page.sned_message_entity.update(cx, |sender, _| {
        sender.group_id = group.id.clone();
    });
    page.history_message_entity.update(cx, |history, _| {
        history.history_message = if clear_history {
            Vec::new()
        } else {
            group.history.clone()
        };
        history.scroll_handle.reset(history.history_message.len());
    });
    page.group_members_entity.update(cx, |members, cx| {
        members.group_users = group.members.clone();
        members.group_type = group.group_type.clone();
        cx.notify();
    });
}
