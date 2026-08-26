use gpui::*;
use gpui_component::VirtualListScrollHandle;
use gpui_component::input::InputState;

use crate::domain::search_entity::{SearchGroupResult, SearchUserResult};

mod core;
mod external;
mod title_bar;
mod ui;

pub use self::external::open;

use self::title_bar::CustomTitleBar;

pub struct AddFriendOrGroupWindow {
    title_bar: Entity<CustomTitleBar>,
    search_input: Entity<InputState>,
    scroll_handler: VirtualListScrollHandle,
    users: Vec<SearchUserResult>,
    groups: Vec<SearchGroupResult>,
    feedback: String,
    pending_friend_id: Option<String>,
    pending_group_id: Option<String>,
    search_generation: u64,
}

impl AddFriendOrGroupWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            title_bar: cx.new(|cx| CustomTitleBar::new(window, cx)),
            search_input: cx.new(|cx| InputState::new(window, cx)),
            scroll_handler: VirtualListScrollHandle::new(),
            users: Vec::new(),
            groups: Vec::new(),
            feedback: String::new(),
            pending_friend_id: None,
            pending_group_id: None,
            search_generation: 0,
        }
    }
}

impl Render for AddFriendOrGroupWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ui::render(self, window, cx)
    }
}
