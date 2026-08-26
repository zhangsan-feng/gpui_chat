use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::avatar::Avatar;
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::label::Label;
use gpui_component::scroll::{Scrollbar, ScrollbarAxis, ScrollbarMode};
use gpui_component::{Root, h_flex, v_flex, v_virtual_list};

use super::{AddFriendOrGroupWindow, core};

pub fn render(
    view: &mut AddFriendOrGroupWindow,
    window: &mut Window,
    cx: &mut Context<AddFriendOrGroupWindow>,
) -> impl IntoElement {
    v_flex()
        .size_full()
        .child(view.title_bar.clone())
        .child(
            v_flex()
                .flex_1()
                .min_h_0()
                .p_5()
                .gap_4()
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_xl()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("添加好友/群聊"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x6b7280))
                                .child("输入用户名、用户 ID、群聊名称或群号，搜索后添加"),
                        ),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .child(Input::new(&view.search_input).flex_1())
                        .child(
                            Button::new("add-friend-window-submit")
                                .label("搜索")
                                .on_click(cx.listener(|view, _, _, cx| core::search(view, cx))),
                        ),
                )
                .when(!view.feedback.is_empty(), |element| {
                    element.child(
                        div()
                            .rounded_md()
                            .bg(rgb(0xf3f4f6))
                            .p_3()
                            .text_sm()
                            .text_color(rgb(0x4b5563))
                            .child(view.feedback.clone()),
                    )
                })
                .child(
                    h_flex()
                        .flex_1()
                        .min_h_0()
                        .child(
                            v_virtual_list(
                                cx.entity(),
                                ("add-friend-window-results", view.search_generation as usize),
                                Rc::new(
                                    view.users
                                        .iter()
                                        .map(|_| ())
                                        .chain(view.groups.iter().map(|_| ()))
                                        .map(|_| size(px(400.), px(64.)))
                                        .collect(),
                                ),
                                |view, visible_range, _window, cx| {
                                    visible_range
                                        .map(|index| {
                                            if index < view.users.len() {
                                                let user = view.users[index].clone();
                                                let pending = view.pending_friend_id.as_deref()
                                                    == Some(user.id.as_str());

                                                h_flex()
                                                    .id(("add-friend-window-user", index))
                                                    .w_full()
                                                    .items_center()
                                                    .justify_between()
                                                    .p_2()
                                                    .rounded_md()
                                                    .hover(|style| style.bg(rgb(0xf3f4f6)))
                                                    .child(
                                                        h_flex()
                                                            .items_center()
                                                            .gap_3()
                                                            .child(
                                                                Avatar::new()
                                                                    .src(user.avatar)
                                                                    .size(px(40.)),
                                                            )
                                                            .child(
                                                                v_flex()
                                                                    .gap_1()
                                                                    .child(Label::new(user.name))
                                                                    .child(
                                                                        div()
                                                                            .text_xs()
                                                                            .text_color(rgb(
                                                                                0x9ca3af,
                                                                            ))
                                                                            .child("好友"),
                                                                    ),
                                                            ),
                                                    )
                                                    .child(
                                                        Button::new((
                                                            "add-friend-window-add",
                                                            index,
                                                        ))
                                                        .label(if pending {
                                                            "发送中"
                                                        } else {
                                                            "添加"
                                                        })
                                                        .on_click(cx.listener(
                                                            move |view, _, window, cx| {
                                                                core::add_friend(
                                                                    view,
                                                                    user.id.clone(),
                                                                    window,
                                                                    cx,
                                                                )
                                                            },
                                                        )),
                                                    )
                                                    .into_any_element()
                                            } else {
                                                let group_index = index - view.users.len();
                                                let group = view.groups[group_index].clone();
                                                let pending = view.pending_group_id.as_deref()
                                                    == Some(group.id.as_str());

                                                h_flex()
                                                    .id(("add-friend-window-group", index))
                                                    .w_full()
                                                    .items_center()
                                                    .justify_between()
                                                    .p_2()
                                                    .rounded_md()
                                                    .hover(|style| style.bg(rgb(0xf3f4f6)))
                                                    .child(
                                                        h_flex()
                                                            .items_center()
                                                            .gap_3()
                                                            .child(
                                                                Avatar::new()
                                                                    .src(group.avatar)
                                                                    .size(px(40.)),
                                                            )
                                                            .child(
                                                                v_flex()
                                                                    .gap_1()
                                                                    .child(Label::new(group.name))
                                                                    .child(
                                                                        div()
                                                                            .text_xs()
                                                                            .text_color(rgb(
                                                                                0x9ca3af,
                                                                            ))
                                                                            .child("群聊"),
                                                                    ),
                                                            ),
                                                    )
                                                    .child(
                                                        Button::new((
                                                            "add-friend-window-join",
                                                            index,
                                                        ))
                                                        .label(if pending {
                                                            "加入中"
                                                        } else {
                                                            "加入"
                                                        })
                                                        .on_click(cx.listener(
                                                            move |view, _, window, cx| {
                                                                core::join_group(
                                                                    view,
                                                                    group.id.clone(),
                                                                    window,
                                                                    cx,
                                                                )
                                                            },
                                                        )),
                                                    )
                                                    .into_any_element()
                                            }
                                        })
                                        .collect()
                                },
                            )
                            .track_scroll(&view.scroll_handler),
                        )
                        .child(
                            Scrollbar::vertical(&view.scroll_handler)
                                .mode(ScrollbarMode::Always)
                                .axis(ScrollbarAxis::Vertical),
                        ),
                ),
        )
        .children(Root::render_dialog_layer(window, cx))
        .children(Root::render_notification_layer(window, cx))
        .children(Root::render_sheet_layer(window, cx))
}
