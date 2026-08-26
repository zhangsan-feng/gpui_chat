use std::path::PathBuf;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Context, InteractiveElement, IntoElement, MouseButton, ParentElement,
    StatefulInteractiveElement, Styled, div, px, rgb,
};
use gpui_component::animation::cubic_bezier;
use gpui_component::avatar::Avatar;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputContentType};
use gpui_component::{Disableable, h_flex, v_flex};

use crate::ui::home::user_profile::UserProfileEditor;

use super::core;

pub fn render(view: &UserProfileEditor, cx: &mut Context<UserProfileEditor>) -> impl IntoElement {
    let user_session = view.state.read(cx).user_state.clone();
    let user_id = if user_session.user_id.is_empty() {
        "未登录".to_string()
    } else {
        user_session.user_id
    };
    let login_name = if user_session.login_name.is_empty() {
        "未设置".to_string()
    } else {
        user_session.login_name
    };

    v_flex()
        .id("home-user-profile")
        .w(px(340.))
        .gap_3()
        .p_3()
        .rounded(px(12.))
        .border_1()
        .border_color(rgb(0xe7dceb))
        .bg(rgb(0xffffff))
        .items_center()
        .child(render_avatar(view, cx))
        .child(render_text_input(
            "用户名",
            Input::new(&view.username)
                .aria_label("用户名")
                .content_type(InputContentType::Username),
        ))
        .child(render_readonly_field("登录名", login_name))
        .child(render_copyable_field("ID", user_id, cx))
        .child(render_text_input(
            "当前密码",
            Input::new(&view.current_password)
                .aria_label("当前密码")
                .content_type(InputContentType::Password)
                .mask_toggle(),
        ))
        .child(render_text_input(
            "新密码",
            Input::new(&view.new_password)
                .aria_label("新密码")
                .content_type(InputContentType::NewPassword)
                .mask_toggle(),
        ))
        .child(render_text_input(
            "确认新密码",
            Input::new(&view.password_confirmation)
                .aria_label("确认新密码")
                .content_type(InputContentType::NewPassword)
                .mask_toggle(),
        ))
        .child(
            h_flex()
                .w_full()
                .gap_2()
                .child(
                    Button::new("home-user-profile-save")
                        .label("保存修改")
                        .primary()
                        .flex_1()
                        .h(px(40.))
                        .loading(view.button_loading)
                        .disabled(view.button_loading)
                        .on_click(cx.listener(|view, _, _, cx| core::submit(view, cx))),
                )
                .child(
                    Button::new("home-user-profile-logout")
                        .label("退出登录")
                        .danger()
                        .flex_1()
                        .h(px(40.))
                        .disabled(view.button_loading)
                        .on_click(
                            cx.listener(|view, _, window, cx| core::logout(view, window, cx)),
                        ),
                ),
        )
        .with_animation(
            "home-user-profile-entrance",
            Animation::new(Duration::from_millis(500))
                .with_easing(cubic_bezier(0.25, 0.1, 0.25, 1.)),
            |this, delta| this.opacity(delta).top(-px(6.) + delta * px(6.)),
        )
}

fn render_avatar(
    view: &UserProfileEditor,
    cx: &mut Context<UserProfileEditor>,
) -> impl IntoElement {
    let user_avatar = view.state.read(cx).user_state.user_avatar.clone();
    let local_avatar_path = view
        .avatar_path
        .as_deref()
        .filter(|path| !path.starts_with("http://") && !path.starts_with("https://"));
    let hint = if view
        .avatar_path
        .as_deref()
        .is_some_and(|path| !path.starts_with("http://") && !path.starts_with("https://"))
    {
        "已选择头像，点击保存生效"
    } else {
        "点击头像更换"
    };
    let mut avatar = div()
        .size(px(96.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .border_2()
        .border_color(rgb(0xbae6fd))
        .overflow_hidden();

    if let Some(path) = local_avatar_path {
        avatar = avatar.child(
            Avatar::new()
                .src(PathBuf::from(path))
                .size_full()
                .into_any_element(),
        );
    } else if user_avatar.is_empty() {
        avatar = avatar.child(div().text_size(px(28.)).child("🙂"));
    } else {
        avatar = avatar.child(
            Avatar::new()
                .src(user_avatar)
                .size_full()
                .into_any_element(),
        );
    }

    v_flex()
        .items_center()
        .gap_1()
        .child(avatar.cursor_pointer().on_mouse_down(
            MouseButton::Left,
            cx.listener(|view, _, _, cx| core::choose_avatar(view, cx)),
        ))
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(0x8b8497))
                .child(hint),
        )
}

fn render_text_input(label: &'static str, input: Input) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_1()
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(0x6f687c))
                .child(label),
        )
        .child(
            div()
                .w_full()
                .h(px(40.))
                .rounded(px(9.))
                .border_1()
                .border_color(rgb(0xe7dceb))
                .bg(rgb(0xfaf7fc))
                .child(input.size_full().appearance(false)),
        )
}

fn render_readonly_field(label: &'static str, value: String) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_1()
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(0x6f687c))
                .child(label),
        )
        .child(
            div()
                .w_full()
                .h(px(40.))
                .flex()
                .items_center()
                .px_3()
                .rounded(px(9.))
                .border_1()
                .border_color(rgb(0xe7dceb))
                .bg(rgb(0xf2eef5))
                .text_size(px(13.))
                .text_color(rgb(0x8b8497))
                .child(value),
        )
}

fn render_copyable_field(
    label: &'static str,
    value: String,
    cx: &mut Context<UserProfileEditor>,
) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_1()
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(0x6f687c))
                .child(label),
        )
        .child(
            div()
                .id("home-user-profile-copy-user-id")
                .w_full()
                .h(px(40.))
                .flex()
                .items_center()
                .px_3()
                .rounded(px(9.))
                .border_1()
                .border_color(rgb(0xe7dceb))
                .bg(rgb(0xf2eef5))
                .text_size(px(13.))
                .text_color(rgb(0x8b8497))
                .cursor_pointer()
                .hover(|mut style| {
                    style.background = Some(rgb(0xece5f0).into());
                    style
                })
                .on_click(cx.listener(|view, _, window, cx| {
                    core::copy_user_id(view, window, cx);
                }))
                .child(value),
        )
}
