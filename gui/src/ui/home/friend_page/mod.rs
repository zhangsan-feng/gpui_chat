use gpui::*;
use gpui_component::VirtualListScrollHandle;

use crate::domain::{MessageGroup, User, WsMsgEvent};

mod core;
mod ui;

#[derive(Clone, Copy, PartialEq, Eq)]
enum FriendTab {
    Friends,
    Groups,
}

pub struct FriendPage {
    user_notice: String,
    group_notice: String,
    friends: Vec<User>,
    groups: Vec<MessageGroup>,
    active_tab: FriendTab,
    friend_scroll_handler: VirtualListScrollHandle,
    group_scroll_handler: VirtualListScrollHandle,
    select_user_id: String,
    select_group_id: String,
}

impl FriendPage {
    pub fn new(_: &mut Context<Self>, _: &mut Window) -> Self {
        Self {
            user_notice: String::new(),
            group_notice: String::new(),
            friends: Vec::new(),
            groups: Vec::new(),
            active_tab: FriendTab::Friends,
            friend_scroll_handler: VirtualListScrollHandle::new(),
            group_scroll_handler: VirtualListScrollHandle::new(),
            select_user_id: String::new(),
            select_group_id: String::new(),
        }
    }

    pub fn init_component_data(
        &mut self,
        friends: Vec<User>,
        groups: Vec<MessageGroup>,
        cx: &mut Context<Self>,
    ) {
        core::init_component_data(self, friends, groups);
        cx.notify();
    }

    pub fn update_component_data(&mut self, event: WsMsgEvent, cx: &mut Context<Self>) {
        core::update_component_data(self, event);
        cx.notify();
    }
}

impl Render for FriendPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ui::render(self, window, cx)
    }
}
