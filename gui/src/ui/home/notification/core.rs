use std::cmp::Reverse;

use gpui::{AsyncApp, Context};
use gpui_tokio::Tokio;
use log::error;
use serde::Deserialize;

use super::NotificationCenter;
use crate::application::notification as notification_application;
use crate::application::session;
use crate::domain::{AppNotification, WsMsgEvent};
use crate::state::GlobalState;

const GROUP_JOIN_REQUEST: &str = "group_join_request";
const GROUP_JOIN_RESULT: &str = "group_join_result";
const FRIEND_REQUEST: &str = "friend_request";

#[derive(Deserialize)]
struct GroupJoinRequestResolvedPayload {
    group_id: String,
    sender_id: String,
    status: String,
    #[serde(default)]
    handled_by_id: String,
    #[serde(default)]
    handled_at: i64,
}

pub fn init_component_data(view: &mut NotificationCenter, notifications: Vec<AppNotification>) {
    view.notifications = notifications
        .into_iter()
        .filter(is_supported_notification)
        .collect();
    sort_notifications(view);
}

pub fn update_component_data(view: &mut NotificationCenter, event: WsMsgEvent) {
    if event.msg_type == "group.join_request_resolved" {
        let Ok(data) = serde_json::from_value::<GroupJoinRequestResolvedPayload>(event.data) else {
            error!("failed to parse resolved group notification event");
            return;
        };
        resolve_pending_notifications(view, data);
        return;
    }

    let notification_event = match event.msg_type.as_str() {
        "group.join_requested"
        | "group.join_request_reviewed"
        | "friend.requested"
        | "friend.request_reviewed" => event,
        _ => return,
    };

    let Ok(notification) = serde_json::from_value::<AppNotification>(notification_event.data)
    else {
        error!("failed to parse group notification event");
        return;
    };
    upsert_notification(view, notification);
}

pub fn set_popover_open(view: &mut NotificationCenter, open: bool) {
    view.popover_open = open;
    if open {
        for notification in &view.notifications {
            view.read_ids.insert(notification.id.clone());
        }
    }
}

pub fn persist_read_changes(
    view: &NotificationCenter,
    read_ids: Vec<String>,
    unread_ids: Vec<String>,
    cx: &mut Context<NotificationCenter>,
) {
    if view.login_name.is_empty() || (read_ids.is_empty() && unread_ids.is_empty()) {
        return;
    }

    let login_name = view.login_name.clone();
    cx.background_executor()
        .spawn(async move {
            if let Err(error) = session::mark_notifications_read(&login_name, &read_ids) {
                error!("failed to persist notification read state: {}", error);
            }
            for notification_id in unread_ids {
                if let Err(error) = session::mark_notification_unread(&login_name, &notification_id)
                {
                    error!("failed to persist notification unread state: {}", error);
                }
            }
        })
        .detach();
}

pub fn review(
    view: &mut NotificationCenter,
    notification_id: String,
    approved: bool,
    cx: &mut Context<NotificationCenter>,
) {
    if view.processing_id.is_some() {
        return;
    }

    let state = cx.global::<GlobalState>().0.read(cx).clone();
    let address = state.http_server;
    let entity = cx.entity().clone();
    let mut async_cx = cx.to_async().clone();
    let Some(notification_type) = view
        .notifications
        .iter()
        .find(|notification| notification.id == notification_id)
        .map(|notification| notification.notification_type.clone())
    else {
        return;
    };
    view.processing_id = Some(notification_id.clone());
    view.feedback.clear();
    cx.notify();

    let task = match notification_type.as_str() {
        FRIEND_REQUEST => Tokio::spawn(
            cx,
            notification_application::review_friend_request(
                address,
                notification_id.clone(),
                approved,
            ),
        ),
        GROUP_JOIN_REQUEST => Tokio::spawn(
            cx,
            notification_application::review_group_join_request(
                address,
                notification_id.clone(),
                approved,
            ),
        ),
        _ => return,
    };
    let is_friend_request = notification_type == FRIEND_REQUEST;

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        match task.await {
            Ok(Ok(result)) => {
                let _ = entity.update(&mut async_cx, |view, cx| {
                    view.processing_id = None;
                    view.feedback = if is_friend_request && result.status == "approved" {
                        "已同意好友申请".to_string()
                    } else if is_friend_request {
                        "已拒绝好友申请".to_string()
                    } else if result.status == "approved" {
                        "已同意入群申请".to_string()
                    } else {
                        "已拒绝入群申请".to_string()
                    };
                    for notification in &mut view.notifications {
                        if notification.id == notification_id
                            || (notification.notification_type == GROUP_JOIN_REQUEST
                                && notification.group_id == result.group_id
                                && notification.sender_id == result.sender_id
                                && notification.status == "pending")
                        {
                            notification.status = result.status.clone();
                            notification.handled_by_id = result.handled_by_id.clone();
                            notification.handled_at = result.handled_at;
                            view.read_ids.insert(notification.id.clone());
                        }
                    }
                    cx.notify();
                });
            }
            Ok(Err(error)) => update_error(&entity, &mut async_cx, error.to_string()),
            Err(error) => update_error(&entity, &mut async_cx, error.to_string()),
        }
    })
    .detach();
}

fn update_error(
    entity: &gpui::Entity<NotificationCenter>,
    async_cx: &mut gpui::AsyncApp,
    message: String,
) {
    let _ = entity.update(async_cx, |view, cx| {
        view.processing_id = None;
        view.feedback = format!("审批失败: {message}");
        cx.notify();
    });
}

fn upsert_notification(view: &mut NotificationCenter, notification: AppNotification) {
    if !is_supported_notification(&notification) {
        return;
    }

    if let Some(existing) = view
        .notifications
        .iter_mut()
        .find(|existing| existing.id == notification.id)
    {
        *existing = notification.clone();
    } else {
        view.notifications.push(notification.clone());
    }
    if view.popover_open {
        view.read_ids.insert(notification.id);
    }
    sort_notifications(view);
}

fn sort_notifications(view: &mut NotificationCenter) {
    view.notifications
        .sort_by_key(|notification| Reverse(notification.created_at));
}

fn resolve_pending_notifications(
    view: &mut NotificationCenter,
    data: GroupJoinRequestResolvedPayload,
) {
    for notification in &mut view.notifications {
        if notification.notification_type != GROUP_JOIN_REQUEST
            || notification.group_id != data.group_id
            || notification.sender_id != data.sender_id
            || notification.status != "pending"
        {
            continue;
        }
        notification.status = data.status.clone();
        notification.handled_by_id = data.handled_by_id.clone();
        notification.handled_at = data.handled_at;
        if view.popover_open {
            view.read_ids.insert(notification.id.clone());
        } else {
            view.read_ids.remove(&notification.id);
        }
    }
}

fn is_supported_notification(notification: &AppNotification) -> bool {
    notification.notification_type == GROUP_JOIN_REQUEST
        || notification.notification_type == GROUP_JOIN_RESULT
        || notification.notification_type == FRIEND_REQUEST
}
