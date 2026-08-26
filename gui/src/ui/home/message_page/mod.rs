use std::collections::HashMap;

use gpui::*;
use gpui_component::VirtualListScrollHandle;
use gpui_component::input::InputState;

use self::group_settings::GroupSettingsPanel;
use self::group_user::GroupMemberEntity;
use self::message::HistoryMessageEntity;
use self::send_message::SendMessageEntity;
use crate::domain::{MessageGroup, WsMsgEvent};

mod core;
mod ui;

mod group_settings;
mod group_user;
mod message;
mod send_message;
mod sidebar;

pub struct MessagePage {
    select_index: usize,
    message_group: Vec<MessageGroup>,
    search_input: Entity<InputState>,
    message_group_scroll_handle: VirtualListScrollHandle,
    unread_counts: HashMap<String, usize>,
    context_menu_group_id: Option<String>,
    left_panel_default_width: f32,
    left_panel_min_width: f32,
    left_panel_max_width: f32,
    history_message_panel_min_height: f32,
    history_message_panel_max_height: f32,
    sned_message_entity: Entity<SendMessageEntity>,
    history_message_entity: Entity<HistoryMessageEntity>,
    group_members_entity: Entity<GroupMemberEntity>,
    group_settings_entity: Entity<GroupSettingsPanel>,
}

#[derive(Clone)]
struct LeftPanelResizeHandle;

impl Render for LeftPanelResizeHandle {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone)]
struct HistoryMessagePanelResizeHandle;

impl Render for HistoryMessagePanelResizeHandle {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl MessagePage {
    pub fn new(cx: &mut Context<Self>, window: &mut Window) -> Self {
        core::new(cx, window)
    }

    pub fn update_component_data(&mut self, event: WsMsgEvent, cx: &mut Context<Self>) {
        core::update_component_data(self, event, cx);
    }

    pub fn init_component_data(&mut self, data: Vec<MessageGroup>, cx: &mut Context<Self>) {
        core::init_component_data(self, data, cx);
    }

    pub fn open_conversation(&mut self, group_id: String, cx: &mut Context<Self>) {
        core::open_conversation(self, group_id, cx);
    }

    pub fn open_private_chat(&mut self, user_id: String, cx: &mut Context<Self>) {
        core::open_private_chat(self, user_id, cx);
    }

    pub fn remove_conversation(&mut self, group_id: String, cx: &mut Context<Self>) {
        core::remove_group(self, &group_id, cx);
    }
}

impl Render for MessagePage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ui::render(self, window, cx)
    }
}
