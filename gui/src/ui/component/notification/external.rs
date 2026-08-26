#![allow(dead_code)]

use gpui::{App, SharedString, Window};
use gpui_component::WindowExt;
use gpui_component::notification::Notification;

pub fn push(window: &mut Window, cx: &mut App, notification: impl Into<Notification>) {
    window.push_notification(notification, cx);
}

pub fn info(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    push(window, cx, Notification::info(message));
}

pub fn success(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    push(window, cx, Notification::success(message));
}

pub fn warning(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    push(window, cx, Notification::warning(message));
}

pub fn error(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    push(window, cx, Notification::error(message));
}
