use std::rc::Rc;

use gpui::*;
use gpui_component::avatar::Avatar;
use gpui_component::label::Label;
use gpui_component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem};
use gpui_component::scroll::{Scrollbar, ScrollbarAxis, ScrollbarMode};
use gpui_component::separator::Separator;
use gpui_component::{h_flex, v_flex, v_virtual_list};

use super::{FriendPage, FriendTab, core};
use crate::state::GlobalState;
use crate::ui::rgb_to_u32;

pub fn render(
    page: &mut FriendPage,
    _window: &mut Window,
    cx: &mut Context<FriendPage>,
) -> impl IntoElement {
    let entity_handle = cx.entity();
    let active_content = match page.active_tab {
        FriendTab::Friends => render_friend_list(page, cx).into_any_element(),
        FriendTab::Groups => render_group_list(page, cx).into_any_element(),
    };

    h_flex()
        .text_size(px(14.))
        .size_full()
        .child(
            v_flex()
                .gap_2()
                .h_full()
                .p_2()
                .w(px(320.))
                .child(
                    h_flex()
                        .w_full()
                        .h(px(32.))
                        .p_1()
                        .gap_1()
                        .rounded(px(6.))
                        .bg(rgb(rgb_to_u32(235, 235, 235)))
                        .child(tab(
                            "friend-page-friends-btn-id",
                            "好友",
                            page.active_tab == FriendTab::Friends,
                            cx.listener(|page, _, _, cx| {
                                page.active_tab = FriendTab::Friends;
                                cx.notify();
                            }),
                        ))
                        .child(tab(
                            "friend-page-groups-btn-id",
                            "群聊",
                            page.active_tab == FriendTab::Groups,
                            cx.listener(|page, _, _, cx| {
                                page.active_tab = FriendTab::Groups;
                                cx.notify();
                            }),
                        )),
                )
                .child(active_content),
        )
        .child(Separator::vertical().h_full())
        .child(
            div()
                .flex_1()
                .size_full()
                .bg(rgb(rgb_to_u32(252, 252, 253))),
        )
        .context_menu(move |menu: PopupMenu, window: &mut Window, popup_cx| {
            let (active_tab, user_id, group_id) = entity_handle.read_with(popup_cx, |page, _| {
                (
                    page.active_tab,
                    page.select_user_id.clone(),
                    page.select_group_id.clone(),
                )
            });

            match active_tab {
                FriendTab::Friends if !user_id.is_empty() => menu
                    .item(PopupMenuItem::new("发送消息").on_click(
                        window.listener_for(&entity_handle, |page, _, _, cx| {
                            core::send_friend_message(page, cx)
                        }),
                    ))
                    .item(PopupMenuItem::new("查看资料").on_click(
                        window.listener_for(&entity_handle, |page, _, window, cx| {
                            core::show_user_profile(page, window, cx)
                        }),
                    ))
                    .item(PopupMenuItem::new("删除好友").on_click(window.listener_for(
                        &entity_handle,
                        |page, _, _, cx| {
                            let friend_id = page.select_user_id.clone();
                            core::delete_friend(page, friend_id, cx);
                        },
                    ))),
                FriendTab::Groups if !group_id.is_empty() => {
                    let current_user_id = popup_cx
                        .global::<GlobalState>()
                        .0
                        .read(popup_cx)
                        .user_state
                        .user_id
                        .clone();
                    let is_owner = entity_handle.read_with(popup_cx, |page, _| {
                        page.groups
                            .iter()
                            .find(|group| group.id == group_id)
                            .is_some_and(|group| {
                                group.members.iter().any(|member| {
                                    member.id == current_user_id && member.user_type == "owner"
                                })
                            })
                    });
                    let action_label = if is_owner {
                        "解散群聊"
                    } else {
                        "退出群聊"
                    };
                    menu.item(PopupMenuItem::new("发送消息").on_click(
                        window.listener_for(&entity_handle, |page, _, _, cx| {
                            core::send_group_message(page, cx)
                        }),
                    ))
                    .item(PopupMenuItem::new("查看群聊").on_click(
                        window.listener_for(&entity_handle, |page, _, window, cx| {
                            core::show_group_info(page, window, cx)
                        }),
                    ))
                    .item(
                        PopupMenuItem::new(action_label).on_click(window.listener_for(
                            &entity_handle,
                            |page, _, _, cx| {
                                let group_id = page.select_group_id.clone();
                                core::leave_group(page, group_id, cx);
                            },
                        )),
                    )
                }
                _ => menu,
            }
        })
}

fn render_friend_list(page: &FriendPage, cx: &mut Context<FriendPage>) -> impl IntoElement {
    h_flex()
        .flex_1()
        .child(
            v_virtual_list(
                cx.entity().clone(),
                "friend-page-friends-list",
                Rc::new(
                    page.friends
                        .iter()
                        .map(|_| size(px(200.), px(58.)))
                        .collect(),
                ),
                |this, visible_range, _, cx| {
                    visible_range
                        .map(|index| {
                            let user = this.friends[index].clone();
                            let user_id_for_context = user.id.clone();
                            let user_id_for_click = user.id.clone();
                            h_flex()
                                .rounded(px(6.))
                                .p_2()
                                .gap_2()
                                .w_full()
                                .id(("friend-page-friend", index))
                                .on_mouse_down(
                                    MouseButton::Right,
                                    cx.listener(move |this, _, _, _| {
                                        this.select_user_id = user_id_for_context.clone();
                                        this.select_group_id.clear();
                                    }),
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.select_user_id = user_id_for_click.clone();
                                    this.select_group_id.clear();
                                    core::send_friend_message(this, cx);
                                }))
                                .hover(|mut style| {
                                    style.background = Some(rgb(rgb_to_u32(226, 226, 226)).into());
                                    style
                                })
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(Avatar::new().src(user.avatar).size(px(40.)))
                                        .child(Label::new(
                                            user.name.chars().take(15).collect::<String>(),
                                        )),
                                )
                        })
                        .collect()
                },
            )
            .track_scroll(&page.friend_scroll_handler),
        )
        .child(
            Scrollbar::vertical(&page.friend_scroll_handler)
                .mode(ScrollbarMode::Always)
                .axis(ScrollbarAxis::Vertical),
        )
}

fn render_group_list(page: &FriendPage, cx: &mut Context<FriendPage>) -> impl IntoElement {
    h_flex()
        .flex_1()
        .child(
            v_virtual_list(
                cx.entity().clone(),
                "friend-page-groups-list",
                Rc::new(
                    page.groups
                        .iter()
                        .map(|_| size(px(200.), px(58.)))
                        .collect(),
                ),
                |this, visible_range, _, cx| {
                    visible_range
                        .map(|index| {
                            let group = this.groups[index].clone();
                            let group_id_for_context = group.id.clone();
                            let group_id_for_click = group.id.clone();
                            h_flex()
                                .rounded(px(6.))
                                .p_2()
                                .gap_2()
                                .w_full()
                                .id(("friend-page-group", index))
                                .on_mouse_down(
                                    MouseButton::Right,
                                    cx.listener(move |this, _, _, _| {
                                        this.select_group_id = group_id_for_context.clone();
                                        this.select_user_id.clear();
                                    }),
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.select_group_id = group_id_for_click.clone();
                                    this.select_user_id.clear();
                                    core::send_group_message(this, cx);
                                }))
                                .hover(|mut style| {
                                    style.background = Some(rgb(rgb_to_u32(226, 226, 226)).into());
                                    style
                                })
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(Avatar::new().src(group.avatar).size(px(40.)))
                                        .child(Label::new(
                                            group.name.chars().take(15).collect::<String>(),
                                        )),
                                )
                        })
                        .collect()
                },
            )
            .track_scroll(&page.group_scroll_handler),
        )
        .child(
            Scrollbar::vertical(&page.group_scroll_handler)
                .mode(ScrollbarMode::Always)
                .axis(ScrollbarAxis::Vertical),
        )
}

fn tab(
    id: &'static str,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .flex_1()
        .rounded(px(5.))
        .p_1()
        .items_center()
        .justify_center()
        .id(id)
        .on_click(on_click)
        .bg(if selected {
            rgb(rgb_to_u32(255, 255, 255))
        } else {
            rgb(rgb_to_u32(235, 235, 235))
        })
        .text_color(if selected {
            rgb(rgb_to_u32(37, 132, 236))
        } else {
            rgb(rgb_to_u32(107, 114, 128))
        })
        .child(label)
}
