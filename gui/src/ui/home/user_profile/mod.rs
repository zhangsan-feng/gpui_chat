use gpui::{AppContext, Context, Entity, Render, Window};
use gpui_component::input::InputState;

use crate::state::State;
use crate::ui::component::dialog;

mod core;
mod ui;

pub struct UserProfileEditor {
    state: Entity<State>,
    username: Entity<InputState>,
    current_password: Entity<InputState>,
    new_password: Entity<InputState>,
    password_confirmation: Entity<InputState>,
    avatar_path: Option<String>,
    button_loading: bool,
    reset_password_inputs: bool,
    feedback: String,
    feedback_dialog_open: bool,
}

impl UserProfileEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, state: Entity<State>) -> Self {
        let username_value = state.read(cx).user_state.username.clone();

        Self {
            username: cx.new(|child_cx| {
                InputState::new(window, child_cx)
                    .default_value(username_value)
                    .placeholder("请输入用户名")
                    .clean_on_escape()
            }),
            current_password: cx.new(|child_cx| {
                InputState::new(window, child_cx)
                    .placeholder("修改密码时填写")
                    .masked(true)
                    .clean_on_escape()
            }),
            new_password: cx.new(|child_cx| {
                InputState::new(window, child_cx)
                    .placeholder("请输入新密码")
                    .masked(true)
                    .clean_on_escape()
            }),
            password_confirmation: cx.new(|child_cx| {
                InputState::new(window, child_cx)
                    .placeholder("请再次输入新密码")
                    .masked(true)
                    .clean_on_escape()
            }),
            state,
            avatar_path: None,
            button_loading: false,
            reset_password_inputs: false,
            feedback: String::new(),
            feedback_dialog_open: false,
        }
    }

    fn open_feedback_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let message = self.feedback.clone();
        let entity = cx.entity().clone();
        self.feedback_dialog_open = true;

        dialog::info_with_close(window, cx, "提示", message, move |_, _, cx| {
            let _ = entity.update(cx, |view, cx| {
                view.feedback.clear();
                view.feedback_dialog_open = false;
                cx.notify();
            });
        });
    }
}

impl Render for UserProfileEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        if self.reset_password_inputs {
            self.reset_password_inputs = false;
            self.current_password
                .update(cx, |input, cx| input.set_value("", window, cx));
            self.new_password
                .update(cx, |input, cx| input.set_value("", window, cx));
            self.password_confirmation
                .update(cx, |input, cx| input.set_value("", window, cx));
        }

        if !self.feedback.is_empty() && !self.feedback_dialog_open {
            self.open_feedback_dialog(window, cx);
        }

        ui::render(self, cx)
    }
}
