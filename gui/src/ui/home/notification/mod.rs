use std::collections::HashSet;

use gpui::{AsyncApp, Context, IntoElement, ListAlignment, ListState, Render, Window, px};

use crate::application::session;
use crate::domain::{AppNotification, WsMsgEvent};

mod core;
mod ui;

pub struct NotificationCenter {
    login_name: String,
    notifications: Vec<AppNotification>,
    scroll_handle: ListState,
    processing_id: Option<String>,
    feedback: String,
    read_ids: HashSet<String>,
    popover_open: bool,
}

impl NotificationCenter {
    pub fn new(login_name: String, cx: &mut Context<Self>) -> Self {
        let entity = cx.entity().clone();
        let mut async_cx = cx.to_async().clone();
        let login_name_for_load = login_name.clone();
        let load_task = cx
            .background_executor()
            .spawn(async move { session::load_notification_read_ids(&login_name_for_load) });
        cx.spawn(move |_, _: &mut AsyncApp| async move {
            match load_task.await {
                Ok(read_ids) => {
                    let _ = entity.update(&mut async_cx, |center, cx| {
                        center.read_ids.extend(read_ids);
                        cx.notify();
                    });
                }
                Err(error) => {
                    log::error!("failed to load notification read state: {}", error);
                }
            }
        })
        .detach();

        Self {
            login_name,
            notifications: Vec::new(),
            scroll_handle: ListState::new(0, ListAlignment::Top, px(112.)),
            processing_id: None,
            feedback: String::new(),
            read_ids: HashSet::new(),
            popover_open: false,
        }
    }

    pub fn init_component_data(
        &mut self,
        notifications: Vec<AppNotification>,
        cx: &mut Context<Self>,
    ) {
        core::init_component_data(self, notifications);
        self.scroll_handle.reset(self.notifications.len());
        cx.notify();
    }

    pub fn update_component_data(&mut self, event: WsMsgEvent, cx: &mut Context<Self>) {
        let previous_count = self.notifications.len();
        let previous_read_ids = self.read_ids.clone();
        let should_remeasure = matches!(
            event.msg_type.as_str(),
            "group.join_requested"
                | "group.join_request_reviewed"
                | "group.join_request_resolved"
                | "friend.requested"
                | "friend.request_reviewed"
        );
        core::update_component_data(self, event);
        let read_ids = self
            .read_ids
            .difference(&previous_read_ids)
            .cloned()
            .collect();
        let unread_ids = previous_read_ids
            .difference(&self.read_ids)
            .cloned()
            .collect();
        core::persist_read_changes(self, read_ids, unread_ids, cx);
        if self.notifications.len() != previous_count {
            self.scroll_handle.reset(self.notifications.len());
        } else if should_remeasure {
            self.scroll_handle.remeasure();
        }
        cx.notify();
    }

    pub fn set_popover_open(&mut self, open: bool, cx: &mut Context<Self>) {
        let previous_read_ids = self.read_ids.clone();
        core::set_popover_open(self, open);
        let read_ids = self
            .read_ids
            .difference(&previous_read_ids)
            .cloned()
            .collect();
        core::persist_read_changes(self, read_ids, Vec::new(), cx);
        cx.notify();
    }

    pub fn unread_count(&self) -> usize {
        self.notifications
            .iter()
            .filter(|notification| !self.read_ids.contains(&notification.id))
            .count()
    }
}

impl Render for NotificationCenter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ui::render(self, window, cx)
    }
}
