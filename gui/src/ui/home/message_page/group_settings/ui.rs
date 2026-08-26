use std::path::PathBuf;

use gpui::prelude::FluentBuilder;
use gpui::{
    Context, InteractiveElement, IntoElement, MouseButton, ParentElement, Styled, Window, div, px,
    rgb,
};
use gpui_component::avatar::Avatar;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::Input;
use gpui_component::{Disableable, h_flex, v_flex};

use super::GroupSettingsPanel;
use super::core;

pub fn render(
    panel: &mut GroupSettingsPanel,
    _window: &mut Window,
    cx: &mut Context<GroupSettingsPanel>,
) -> impl IntoElement {
    let avatar_path = panel.avatar_path.clone();
    let avatar_url = panel.group_avatar.clone();
    let can_edit = panel.can_edit;
    let saving = panel.saving;
    let hint = if can_edit {
        if avatar_path.is_some() {
            "已选择头像，保存后生效"
        } else {
            "点击头像更换"
        }
    } else {
        "仅群主可编辑"
    };

    v_flex()
        .id("group-settings-panel")
        .w(px(320.))
        .gap_3()
        .p_4()
        .rounded(px(12.))
        .bg(rgb(0xffffff))
        .child(
            div()
                .text_size(px(16.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child("群聊设置"),
        )
        .child(render_avatar(avatar_path, avatar_url, can_edit, hint, cx))
        .child(render_text_input(
            "群聊名称",
            Input::new(&panel.group_name)
                .aria_label("群聊名称")
                .disabled(!can_edit || saving),
        ))
        .child(
            Checkbox::new("group-settings-allow-join")
                .label("允许加入群聊")
                .checked(panel.allow_join)
                .disabled(!can_edit || saving)
                .on_click(cx.listener(|panel, checked, _, cx| {
                    core::toggle_allow_join(panel, checked, cx);
                })),
        )
        .child(
            h_flex().w_full().justify_end().child(
                Button::new("group-settings-save")
                    .label("保存")
                    .primary()
                    .loading(saving)
                    .disabled(!can_edit || saving)
                    .on_click(cx.listener(|panel, _, _, cx| core::submit(panel, cx))),
            ),
        )
        .when(!panel.feedback.is_empty(), |this| {
            this.child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(0x6f687c))
                    .child(panel.feedback.clone()),
            )
        })
}

fn render_avatar(
    avatar_path: Option<PathBuf>,
    avatar_url: String,
    can_edit: bool,
    hint: &'static str,
    cx: &mut Context<GroupSettingsPanel>,
) -> impl IntoElement {
    let mut avatar = div()
        .size(px(72.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .border_2()
        .border_color(rgb(0xbae6fd))
        .overflow_hidden();

    if let Some(path) = avatar_path {
        avatar = avatar.child(Avatar::new().src(path).size_full().into_any_element());
    } else if avatar_url.is_empty() {
        avatar = avatar.child(div().text_size(px(24.)).child("👥"));
    } else {
        avatar = avatar.child(Avatar::new().src(avatar_url).size_full().into_any_element());
    }

    v_flex()
        .items_center()
        .gap_1()
        .child(avatar.when(can_edit, |this| {
            this.cursor_pointer().on_mouse_down(
                MouseButton::Left,
                cx.listener(|panel, _, _, cx| core::choose_avatar(panel, cx)),
            )
        }))
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
                .h(px(36.))
                .rounded(px(9.))
                .border_1()
                .border_color(rgb(0xe7dceb))
                .bg(rgb(0xfaf7fc))
                .child(input.size_full().appearance(false)),
        )
}
