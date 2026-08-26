use chrono::{Local, NaiveDateTime, TimeZone};
use gpui::*;
use gpui_tokio::Tokio;
use std::rc::Rc;

use super::super::MessagePage;
use super::super::core;
use super::create_group_window::CreateGroupChatWindow;
use crate::application::group as group_application;
use gpui_component::avatar::Avatar;
use gpui_component::button::Button;
use gpui_component::input::Input;
use gpui_component::label::Label;
use gpui_component::menu::{ContextMenuExt, DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_component::scroll::{Scrollbar, ScrollbarAxis, ScrollbarMode};
use gpui_component::{Root, Sizable, StyledExt, h_flex, v_flex, v_virtual_list};

use crate::state::GlobalState;
use crate::ui::component::window::window_center_options;
use crate::ui::home::add_friend_or_group_window;
use crate::ui::rgb_to_u32;

fn format_message_time(time_str: &str) -> String {
    let date_time =
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S") {
            Local.from_local_datetime(&naive_dt).single()
        } else if let Ok(timestamp) = time_str.parse::<i64>() {
            if timestamp.abs() >= 10_000_000_000 {
                Local.timestamp_millis_opt(timestamp).single()
            } else {
                Local.timestamp_opt(timestamp, 0).single()
            }
        } else {
            return time_str.to_string();
        };

    let Some(date_time) = date_time else {
        return String::new();
    };

    if date_time.date_naive() == Local::now().date_naive() {
        date_time.format("%H:%M").to_string()
    } else {
        date_time.format("%m-%d").to_string()
    }
}

fn group_action(group_id: String, disband: bool, cx: &mut Context<MessagePage>) {
    let address = cx
        .global::<GlobalState>()
        .0
        .clone()
        .read(cx)
        .http_server
        .clone();
    let entity = cx.entity();
    let mut async_cx = cx.to_async();
    let request_group_id = group_id.clone();
    let task = Tokio::spawn(cx, async move {
        if disband {
            group_application::disband_group(address, request_group_id).await
        } else {
            group_application::leave_group(address, request_group_id).await
        }
    });

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        let result = task.await;

        match result {
            Ok(Ok(())) => {
                let _ = entity.update(&mut async_cx, |page, cx| {
                    core::remove_group(page, &group_id, cx);
                });
            }
            Ok(Err(error)) => {
                log::error!("group action failed: {}", error);
            }
            Err(error) => {
                log::error!("group action task failed: {}", error);
            }
        }
    })
    .detach();
}

impl MessagePage {
    fn left_sidebar_vm_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_virtual_list(
            cx.entity().clone(),
            "left_sidebar_vm_list",
            Rc::new(
                self.message_group
                    .iter()
                    .map(|_| size(px(200.), px(70.)))
                    .collect(),
            ),
            |this, visible_range, window, cx| {
                visible_range
                    .map(|vm_index| {
                        let global_state = cx.global::<GlobalState>().0.clone().read(cx);
                        let index = this.select_index.clone();
                        let group = this.message_group[vm_index].clone();
                        let mut group_avatar = group.avatar.clone();

                        let name = group.name.clone();

                        let max_message_chars = 15;
                        let mut group_name = name;

                        let group_id = group.id.clone();
                        let context_menu_group_id = if group.group_type == "group" {
                            Some(group_id.clone())
                        } else {
                            None
                        };
                        let last_message = group.history.last().cloned().unwrap_or_default();

                        let mut last_message_ui = format!(
                            "{}:{}",
                            last_message.send_username,
                            last_message
                                .message
                                .replace("\n", "")
                                .chars()
                                .take(max_message_chars)
                                .collect::<String>()
                        );
                        if last_message_ui == ":" {
                            last_message_ui = "".to_string();
                        }

                        let message_time = format_message_time(&last_message.time);
                        let unread_count = this
                            .unread_counts
                            .get(&group_id)
                            .copied()
                            .unwrap_or_default();
                        let unread_label = if unread_count > 99 {
                            "99+".to_string()
                        } else {
                            unread_count.to_string()
                        };

                        if group.group_type == "private_chat" {
                            group.members.iter().for_each(|x| {
                                if x.id != global_state.user_state.user_id {
                                    group_avatar = x.avatar.clone();
                                    group_name = x.name.clone();
                                }
                            })
                        }

                        h_flex()
                            .id(("message-group-vm-list", vm_index))
                            .size_full()
                            .hover(|mut style| {
                                if index != vm_index {
                                    style.background = Some(rgb(rgb_to_u32(226, 226, 226)).into());
                                }
                                style
                            })
                            .rounded(px(12.))
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |this, _, _, cx| {
                                    this.context_menu_group_id = context_menu_group_id.clone();
                                    cx.notify();
                                }),
                            )
                            .on_click({
                                cx.listener({
                                    move |this, _, _, cx| {
                                        this.select_index = vm_index.clone();
                                        let group_data =
                                            this.message_group[this.select_index].clone();

                                        let group_id =
                                            this.message_group[this.select_index].id.clone();
                                        this.unread_counts.insert(group_id.clone(), 0);

                                        this.sned_message_entity.update(cx, |this, cx| {
                                            this.group_id = group_id.clone();
                                        });

                                        this.history_message_entity.update(cx, |this, cx| {
                                            this.history_message = group_data.history.clone();
                                            this.scroll_handle.reset(group_data.history.len());
                                            cx.notify();
                                        });

                                        this.group_members_entity.update(cx, |this, cx| {
                                            this.group_users = group_data.members.clone();
                                            this.group_type = group_data.group_type.clone();
                                            cx.notify();
                                        });
                                    }
                                })
                            })
                            .bg(if index == vm_index {
                                rgb(rgb_to_u32(226, 226, 226))
                            } else {
                                rgb(rgb_to_u32(255, 255, 255))
                            })
                            .child(
                                h_flex()
                                    .size_full()
                                    .gap_2()
                                    .p_2()
                                    .child(
                                        Avatar::new()
                                            .src(group_avatar)
                                            .with_size(gpui_component::Size::Size(px(40.0))),
                                    )
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap_1()
                                            .child(
                                                h_flex()
                                                    .w_full()
                                                    .items_center()
                                                    .justify_between()
                                                    .min_w_0()
                                                    .child(
                                                        div()
                                                            .w(relative(0.5))
                                                            .flex_shrink_0()
                                                            .min_w_0()
                                                            .truncate()
                                                            .child(group_name),
                                                    )
                                                    .child(
                                                        div()
                                                            .ml_2()
                                                            .flex_shrink_0()
                                                            .text_size(px(12.))
                                                            .text_color(rgb(0x9ca3af))
                                                            .child(message_time),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .w_full()
                                                    .items_center()
                                                    .justify_between()
                                                    .min_w_0()
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .min_w_0()
                                                            .overflow_hidden()
                                                            .truncate()
                                                            .text_size(px(12.))
                                                            .text_color(rgb(0x9ca3af))
                                                            .child(last_message_ui),
                                                    )
                                                    .child(if unread_count == 0 {
                                                        div().flex_shrink_0()
                                                    } else {
                                                        div()
                                                            .flex_shrink_0()
                                                            .paddings(px(2.0))
                                                            .text_size(px(11.))
                                                            .text_center()
                                                            .child(Label::new(unread_label.clone()))
                                                            .bg(rgb(0xc7c7c7))
                                                            .rounded_full()
                                                    }),
                                            ),
                                    ),
                            )
                    })
                    .collect()
            },
        )
        .h_full()
        .track_scroll(&self.message_group_scroll_handle)
    }

    fn add_btn_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity_handle = cx.entity().clone();
        Button::new("left-sidebar-btn-menu")
            .label("+")
            .mx_2()
            .dropdown_menu_with_anchor(Anchor::TopLeft, move |menu, window, cx| {
                menu.item(PopupMenuItem::new("创建群聊").on_click(window.listener_for(
                    &entity_handle.clone(),
                    |this, event, window, cx| {
                        let _ = cx.open_window(
                            window_center_options(window, size(px(420.), px(380.))),
                            move |window, app| {
                                let view = app.new(|cx| CreateGroupChatWindow::new(cx, window));
                                app.new(|cx| Root::new(view, window, cx))
                            },
                        );
                    },
                )))
                .item(
                    PopupMenuItem::new("添加好友/群聊").on_click(window.listener_for(
                        &entity_handle,
                        |this, event, window, cx| {
                            if let Err(error) = add_friend_or_group_window::open(window, cx) {
                                log::error!("failed to open add friend/group window: {}", error);
                            }
                        },
                    )),
                )
            })
    }

    pub fn left_sidebar(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity_handle = cx.entity().clone();
        v_flex()
            .size_full()
            .w(px(self.left_panel_default_width))
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .p_2()
                    .child(
                        Input::new(&self.search_input)
                            .flex_1()
                            .h(px(32.))
                            .appearance(false)
                            .bg(rgb(rgb_to_u32(237, 237, 237)))
                            .rounded(px(8.))
                            .px_2(),
                    )
                    .child(self.add_btn_menu(cx)),
            )
            .child(
                h_flex()
                    .size_full()
                    .child(self.left_sidebar_vm_list(cx))
                    .child(
                        Scrollbar::vertical(&self.message_group_scroll_handle)
                            .mode(ScrollbarMode::Always)
                            .axis(ScrollbarAxis::Vertical),
                    ),
            )
            .context_menu(
                move |menu: PopupMenu, w: &mut Window, cx: &mut Context<PopupMenu>| {
                    let context_menu_group_id =
                        entity_handle.read_with(cx, |page, _| page.context_menu_group_id.clone());
                    let Some(group_id) = context_menu_group_id.clone() else {
                        return menu;
                    };
                    let current_user_id = cx
                        .global::<GlobalState>()
                        .0
                        .clone()
                        .read(cx)
                        .user_state
                        .user_id
                        .clone();
                    let is_group_owner = entity_handle.read_with(cx, |page, _| {
                        page.message_group
                            .iter()
                            .find(|group| group.id == group_id)
                            .is_some_and(|group| {
                                group.members.iter().any(|member| {
                                    member.id == current_user_id && member.user_type == "owner"
                                })
                            })
                    });
                    let copy_group_id = group_id.clone();
                    let action_group_id = group_id;
                    menu.item(PopupMenuItem::new("复制群号").on_click(w.listener_for(
                        &entity_handle,
                        move |_, _, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy_group_id.clone()));
                        },
                    )))
                    .item(
                        PopupMenuItem::new(if is_group_owner {
                            "解散群聊"
                        } else {
                            "退出群聊"
                        })
                        .on_click(w.listener_for(
                            &entity_handle,
                            move |_, _, _, cx| {
                                group_action(action_group_id.clone(), is_group_owner, cx);
                            },
                        )),
                    )
                },
            )
    }
}
