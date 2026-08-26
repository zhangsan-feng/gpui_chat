use gpui::prelude::FluentBuilder;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    Styled, Window, div, px, rgb,
};
use gpui_component::animation::cubic_bezier;
use gpui_component::avatar::Avatar;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::scroll::{Scrollbar, ScrollbarAxis, ScrollbarMode};
use gpui_component::{Disableable, h_flex, v_flex};

use super::NotificationCenter;
use super::core;
use crate::domain::AppNotification;

const GROUP_JOIN_REQUEST: &str = "group_join_request";
const GROUP_JOIN_RESULT: &str = "group_join_result";
const FRIEND_REQUEST: &str = "friend_request";

pub fn render(
    view: &mut NotificationCenter,
    _window: &mut Window,
    cx: &mut Context<NotificationCenter>,
) -> impl IntoElement {
    let notifications = view.notifications.clone();
    let processing_id = view.processing_id.clone();
    let feedback = view.feedback.clone();
    let entity = cx.entity().clone();

    v_flex()
        .id("home-notification-center")
        .w(px(380.))
        .max_h(px(520.))
        .gap_2()
        .p_1()
        .child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(0x49425c))
                        .child("通知"),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(0x8a8294))
                        .child(format!("{} 条", notifications.len())),
                ),
        )
        .when(!feedback.is_empty(), |this| {
            this.child(
                div()
                    .w_full()
                    .rounded(px(6.))
                    .bg(rgb(0xf0fdf4))
                    .text_color(rgb(0x34745b))
                    .p_2()
                    .text_size(px(12.))
                    .child(feedback.clone()),
            )
        })
        .child(if notifications.is_empty() {
            div()
                .w_full()
                .p_4()
                .items_center()
                .justify_center()
                .text_color(rgb(0x8a8294))
                .child("暂无通知")
                .into_any_element()
        } else {
            h_flex()
                .w_full()
                .h(px(440.))
                .min_h_0()
                .gap_1()
                .child(
                    gpui::list(view.scroll_handle.clone(), move |index, _, _| {
                        render_notification(
                            notifications[index].clone(),
                            processing_id.as_deref(),
                            entity.clone(),
                        )
                        .into_any_element()
                    })
                    .size_full()
                    .p_1()
                    .into_any_element(),
                )
                .child(
                    Scrollbar::vertical(&view.scroll_handle)
                        .mode(ScrollbarMode::Always)
                        .axis(ScrollbarAxis::Vertical),
                )
                .into_any_element()
        })
        .with_animation(
            "home-notification-center-entrance",
            Animation::new(Duration::from_millis(500))
                .with_easing(cubic_bezier(0.25, 0.1, 0.25, 1.)),
            |this, delta| this.opacity(delta).top(-px(6.) + delta * px(6.)),
        )
}

fn render_notification(
    notification: AppNotification,
    processing_id: Option<&str>,
    entity: Entity<NotificationCenter>,
) -> impl IntoElement {
    let is_request = notification.notification_type == GROUP_JOIN_REQUEST;
    let is_result = notification.notification_type == GROUP_JOIN_RESULT;
    let is_friend_request = notification.notification_type == FRIEND_REQUEST;
    let pending = notification.status == "pending";
    let processing = processing_id == Some(notification.id.as_str());
    let title = if is_friend_request && pending {
        "新的好友申请"
    } else if is_friend_request && notification.status == "approved" {
        "好友申请已通过"
    } else if is_friend_request {
        "好友申请已拒绝"
    } else if is_request && pending {
        "新的入群申请"
    } else if is_result && notification.status == "approved" {
        "入群申请已通过"
    } else if is_result {
        "入群申请已拒绝"
    } else if notification.status == "approved" {
        "入群申请已同意"
    } else {
        "入群申请已拒绝"
    };
    let subject_name = if is_friend_request && !pending {
        display_name(&notification.recipient_name, &notification.recipient_id)
    } else {
        display_name(&notification.sender_name, &notification.sender_id)
    };
    let subject_avatar = if is_friend_request && !pending {
        notification.recipient_avatar.clone()
    } else {
        notification.sender_avatar.clone()
    };
    let description = if is_friend_request && pending {
        "请求添加你为好友".to_string()
    } else if is_friend_request && notification.status == "approved" {
        format!("你与 {} 已成为好友", subject_name)
    } else if is_friend_request {
        format!("你与 {} 的好友申请未通过", subject_name)
    } else if is_request && pending {
        "申请加入群聊".to_string()
    } else if is_result && notification.status == "approved" {
        format!(
            "你已加入群聊 {}",
            display_name(&notification.group_name, &notification.group_id)
        )
    } else if is_result {
        format!(
            "你加入群聊 {} 的申请未通过",
            display_name(&notification.group_name, &notification.group_id)
        )
    } else if notification.status == "approved" {
        "已同意加入群聊".to_string()
    } else {
        "已拒绝加入群聊".to_string()
    };
    let notification_id = notification.id.clone();
    let approve_id = notification_id.clone();
    let reject_id = notification_id.clone();
    let approve_element_id = format!("notification-approve-{approve_id}");
    let reject_element_id = format!("notification-reject-{reject_id}");
    let approve_entity = entity.clone();
    let reject_entity = entity;
    let approve_button = Button::new(approve_element_id)
        .label("同意")
        .primary()
        .flex_1()
        .h(px(30.))
        .loading(processing)
        .disabled(processing)
        .on_click(move |_, _, app| {
            let _ = approve_entity.update(app, |view, cx| {
                core::review(view, approve_id.clone(), true, cx);
            });
        });
    let reject_button = Button::new(reject_element_id)
        .label("拒绝")
        .flex_1()
        .h(px(30.))
        .disabled(processing)
        .on_click(move |_, _, app| {
            let _ = reject_entity.update(app, |view, cx| {
                core::review(view, reject_id.clone(), false, cx);
            });
        });
    let actions = h_flex()
        .w_full()
        .gap_2()
        .pt_1()
        .child(approve_button)
        .child(reject_button);

    v_flex()
        .w_full()
        .gap_1()
        .rounded(px(8.))
        .border_1()
        .border_color(rgb(0xe7dceb))
        .bg(if pending {
            rgb(0xfffbeb)
        } else {
            rgb(0xffffff)
        })
        .p_2()
        .child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(0x49425c))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(0x8a8294))
                        .child(status_label(&notification)),
                ),
        )
        .child(render_notification_subject(
            &subject_name,
            &subject_avatar,
            &notification.group_name,
            &notification.group_avatar,
            &notification.group_id,
        ))
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(0x6b6375))
                .child(description),
        )
        .when((is_request || is_friend_request) && pending, |this| {
            this.child(actions)
        })
}

fn render_notification_subject(
    subject_name: &str,
    subject_avatar: &str,
    group_name: &str,
    group_avatar: &str,
    group_id: &str,
) -> impl IntoElement {
    let group_name = if group_name.is_empty() {
        group_id.to_string()
    } else {
        group_name.to_string()
    };
    h_flex()
        .w_full()
        .items_center()
        .gap_2()
        .child(
            Avatar::new()
                .when(!subject_avatar.is_empty(), |this| {
                    this.src(subject_avatar.to_string())
                })
                .size(px(34.)),
        )
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .truncate()
                        .child(subject_name.to_string()),
                )
                .when(!group_id.is_empty(), |this| {
                    this.child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child(
                                Avatar::new()
                                    .when(!group_avatar.is_empty(), |this| {
                                        this.src(group_avatar.to_string())
                                    })
                                    .size(px(20.)),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(0x8a8294))
                                    .truncate()
                                    .child(format!("群聊：{}", group_name)),
                            ),
                    )
                }),
        )
}

fn display_name(name: &str, id: &str) -> String {
    if name.is_empty() {
        id.to_string()
    } else {
        name.to_string()
    }
}

fn status_label(notification: &AppNotification) -> &'static str {
    match notification.status.as_str() {
        "pending" => "待处理",
        "approved" => "已通过",
        "rejected" => "已拒绝",
        "cancelled" => "已取消",
        _ => "已处理",
    }
}
