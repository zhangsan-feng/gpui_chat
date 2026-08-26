use gpui::{AppContext, Context, Entity, Render, SharedString, Window};
use gpui_component::VirtualListScrollHandle;
use gpui_component::input::InputState;

use crate::application::session;
use crate::domain::LoginResponseMsg;
use crate::ui::component::dialog;

mod core;
mod external;
mod title_bar;
mod ui;

pub use self::external::open;

use self::title_bar::CustomTitleBar;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    AccountSelection,
    Login,
    Register,
}

pub struct LoginView {
    title_bar: Entity<CustomTitleBar>,
    login_name: Entity<InputState>,
    password: Entity<InputState>,
    password_confirmation: Entity<InputState>,
    account_scroll_handler: VirtualListScrollHandle,
    avatar_path: Option<SharedString>,
    button_loading: bool,
    feedback: String,
    current_page: Page,
    saved_sessions: Vec<LoginResponseMsg>,
    open_home_pending: bool,
}

impl LoginView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (saved_sessions, feedback) = match session::load_saved_sessions() {
            Ok(sessions) => (sessions, String::new()),
            Err(error) => (Vec::new(), format!("加载已登录账号失败: {error}")),
        };

        Self {
            title_bar: cx.new(|title_bar_cx| CustomTitleBar::new(window, title_bar_cx)),
            login_name: cx.new(|child_cx| {
                InputState::new(window, child_cx)
                    .placeholder("请输入登录名")
                    .clean_on_escape()
            }),
            password: cx.new(|child_cx| {
                InputState::new(window, child_cx)
                    .placeholder("请输入密码")
                    .masked(true)
                    .clean_on_escape()
            }),
            password_confirmation: cx.new(|child_cx| {
                InputState::new(window, child_cx)
                    .placeholder("请再次输入密码")
                    .masked(true)
                    .clean_on_escape()
            }),
            account_scroll_handler: VirtualListScrollHandle::new(),
            avatar_path: None,
            button_loading: false,
            feedback,
            current_page: if saved_sessions.is_empty() {
                Page::Login
            } else {
                Page::AccountSelection
            },
            saved_sessions,
            open_home_pending: false,
        }
    }
}

impl Render for LoginView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        if self.open_home_pending {
            self.open_home_pending = false;
            if let Err(error) = crate::ui::home::HomeView::open(cx) {
                self.feedback = format!("无法打开主页面: {error}");
            } else {
                window.remove_window();
            }
        }

        if !self.feedback.is_empty()
            && !dialog::is_open(window, cx)
            && self.feedback != "正在恢复登录状态…"
        {
            self.open_feedback_dialog(window, cx);
        }

        ui::render(self, window, cx)
    }
}

impl LoginView {
    fn open_feedback_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let message = self.feedback.clone();
        let entity = cx.entity().clone();

        dialog::info_with_close(window, cx, "提示", message, move |_, _, cx| {
            let _ = entity.update(cx, |view, cx| {
                view.feedback.clear();
                cx.notify();
            });
        });
    }
}
