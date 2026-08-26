use std::path::PathBuf;

use gpui::{AppContext, Context, Entity, Render, Window};
use gpui_component::input::InputState;

use crate::domain::MessageGroup;

mod core;
mod ui;

pub struct GroupSettingsPanel {
    group_id: String,
    group_name: Entity<InputState>,
    group_avatar: String,
    avatar_path: Option<PathBuf>,
    allow_join: bool,
    can_edit: bool,
    saving: bool,
    feedback: String,
}

impl GroupSettingsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            group_id: String::new(),
            group_name: cx.new(|cx| InputState::new(window, cx).placeholder("请输入群聊名称")),
            group_avatar: String::new(),
            avatar_path: None,
            allow_join: true,
            can_edit: false,
            saving: false,
            feedback: String::new(),
        }
    }

    pub fn sync_group(
        &mut self,
        group: &MessageGroup,
        current_user_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        core::sync_group(self, group, current_user_id, window, cx);
    }
}

impl Render for GroupSettingsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        ui::render(self, window, cx)
    }
}
