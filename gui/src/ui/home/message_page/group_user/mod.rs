use crate::application::group::{create_private_chat, update_member_role};
use crate::application::social;
use crate::domain::GroupMembers;
use crate::state::{EventBus, GlobalState};
use crate::ui::{avatar_source, rgb_to_u32};
use gpui::{
    AppContext, AsyncApp, Context, Entity, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Render, Styled, Window, div, px, rgb, size,
};
use gpui_component::avatar::Avatar;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::label::Label;
use gpui_component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem};
use gpui_component::scroll::{Scrollbar, ScrollbarAxis, ScrollbarMode};
use gpui_component::separator::Separator;
use gpui_component::{Sizable, VirtualListScrollHandle, h_flex, v_flex, v_virtual_list};
use gpui_tokio::Tokio;
use std::rc::Rc;

fn member_role_priority(role: &str) -> u8 {
    match role {
        "owner" => 0,
        "admin" => 1,
        _ => 2,
    }
}

fn member_role_label(role: &str) -> Option<(&'static str, u32)> {
    match role {
        "owner" => Some(("群主", 0xca8a04)),
        "admin" => Some(("管理员", 0x16a34a)),
        _ => None,
    }
}

fn change_member_role(
    entity: &mut GroupMemberEntity,
    role: &'static str,
    cx: &mut Context<GroupMemberEntity>,
) {
    let Some(member) = entity
        .group_users
        .iter()
        .find(|member| member.id == entity.select_user_id)
        .cloned()
    else {
        return;
    };
    if member.user_type == "owner" {
        return;
    }

    let global_state = cx.global::<GlobalState>().0.clone();
    let global_state_read = global_state.read(cx).clone();
    let address = global_state_read.http_server;
    let group_id = member.group_id;
    let user_id = member.id;
    let role = role.to_string();
    let task = Tokio::spawn(cx, update_member_role(address, group_id, user_id, role));

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        let result = task.await;
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => log::error!("更新群成员角色失败: {}", error),
            Err(error) => log::error!("更新群成员角色任务失败: {}", error),
        }
    })
    .detach();
}

fn request_friend(entity: &mut GroupMemberEntity, cx: &mut Context<GroupMemberEntity>) {
    let Some(member) = entity
        .group_users
        .iter()
        .find(|member| member.id == entity.select_user_id)
        .cloned()
    else {
        return;
    };

    let global_state = cx.global::<GlobalState>().0.clone();
    let state = global_state.read(cx).clone();
    let task = Tokio::spawn(
        cx,
        social::add_friend(state.http_server, state.user_state.user_id, member.id),
    );

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        match task.await {
            Ok(Ok(())) => log::info!("好友申请已发送"),
            Ok(Err(error)) => log::error!("发送好友申请失败: {}", error),
            Err(error) => log::error!("发送好友申请任务失败: {}", error),
        }
    })
    .detach();
}

pub struct GroupMemberEntity {
    pub group_type: String,
    pub group_users: Vec<GroupMembers>,
    search_input: Entity<InputState>,
    scroll_handle: VirtualListScrollHandle,
    select_user_id: String,
}

impl GroupMemberEntity {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("搜索群成员"));
        cx.subscribe(&search_input, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        })
        .detach();

        GroupMemberEntity {
            group_type: "".to_string(),
            group_users: vec![],
            search_input,
            scroll_handle: VirtualListScrollHandle::new(),
            select_user_id: Default::default(),
        }
    }
}

impl Render for GroupMemberEntity {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity_handle = cx.entity().clone();

        if self.group_users.len() != 0 && self.group_type == "private_chat" {
            div().into_any_element()
        } else {
            let current_user_id = cx
                .global::<GlobalState>()
                .0
                .clone()
                .read(cx)
                .user_state
                .user_id
                .clone();
            let can_manage_roles = self.group_type == "group"
                && self
                    .group_users
                    .iter()
                    .any(|member| member.id == current_user_id && member.user_type == "owner");
            let search_keyword = self.search_input.read(cx).value().to_string();
            let search_keyword = search_keyword.trim().to_lowercase();
            let mut filtered_group_users = self
                .group_users
                .iter()
                .filter(|member| {
                    search_keyword.is_empty()
                        || member.name.to_lowercase().contains(&search_keyword)
                })
                .cloned()
                .collect::<Vec<_>>();
            filtered_group_users.sort_by_key(|member| member_role_priority(&member.user_type));
            v_flex()
                .id("group_member")
                .mb_2()
                .h_full()
                .w(px(240.))
                .child(
                    v_flex()
                        .size_full()
                        .child(Label::new("群成员").p_2())
                        .child(
                            div().px_2().pb_2().child(
                                Input::new(&self.search_input)
                                    .h(px(32.))
                                    .appearance(false)
                                    .bg(rgb(rgb_to_u32(237, 237, 237)))
                                    .rounded(px(8.))
                                    .px_2(),
                            ),
                        )
                        .child(Separator::horizontal().w_full())
                        .child(
                            v_flex()
                                .gap_2()
                                .size_full()
                                .child(
                                    v_virtual_list(
                                        cx.entity().clone(),
                                        "group_member_component_vm_list",
                                        Rc::new(
                                            filtered_group_users
                                                .iter()
                                                .map(|_| size(px(240.), px(40.)))
                                                .collect(),
                                        ),
                                        move |_view, visible_range, _, cx| {
                                            visible_range
                                                .map(|index| {
                                                    let group_user =
                                                        filtered_group_users[index].clone();
                                                    let group_user_id = group_user.id.clone();
                                                    let selected_user_id = group_user_id.clone();
                                                    h_flex()
                                                        .on_mouse_down(
                                                            MouseButton::Right,
                                                            cx.listener(move |this, _, _, cx| {
                                                                this.select_user_id =
                                                                    selected_user_id.clone();
                                                            }),
                                                        )
                                                        .w_full()
                                                        .h(px(40.))
                                                        .p_2()
                                                        .id(("group-member-component", index))
                                                        .rounded(px(4.0))
                                                        .size_full()
                                                        .hover(|mut style| {
                                                            style.background = Some(
                                                                rgb(rgb_to_u32(235, 235, 235))
                                                                    .into(),
                                                            );
                                                            style
                                                        })
                                                        .rounded(px(5.0))
                                                        .child(
                                                            Avatar::new()
                                                                .src(avatar_source(
                                                                    &group_user.avatar,
                                                                    &group_user_id,
                                                                ))
                                                                .with_size(px(30.)),
                                                        )
                                                        .child(
                                                            Label::new(
                                                                group_user
                                                                    .name
                                                                    .chars()
                                                                    .take(10)
                                                                    .collect::<String>(),
                                                            )
                                                            .flex_1(),
                                                        )
                                                        .child(div().flex_grow_1())
                                                        .child(
                                                            match member_role_label(
                                                                &group_user.user_type,
                                                            ) {
                                                                Some((label, color)) => {
                                                                    Label::new(label)
                                                                        .text_color(rgb(color))
                                                                        .into_any_element()
                                                                }
                                                                None => div().into_any_element(),
                                                            },
                                                        )
                                                })
                                                .collect()
                                        },
                                    )
                                    .track_scroll(&self.scroll_handle)
                                    .w_full()
                                    .flex_1(),
                                )
                                .child(
                                    Scrollbar::vertical(&self.scroll_handle)
                                        .mode(ScrollbarMode::Always)
                                        .axis(ScrollbarAxis::Vertical),
                                ),
                        ),
                )
                .context_menu(
                    move |menu: PopupMenu, w: &mut Window, _cx: &mut Context<PopupMenu>| {
                        let selected_user_id =
                            entity_handle.read_with(_cx, |entity, _| entity.select_user_id.clone());
                        let can_add_friend =
                            !selected_user_id.is_empty() && selected_user_id != current_user_id;
                        let menu = menu.item(PopupMenuItem::new("发消息").on_click(
                            w.listener_for(&entity_handle, |this, _, _, cx| {
                                let global_state = cx.global::<GlobalState>().0.clone();
                                let global_state_read = global_state.read(cx).clone();
                                let address = global_state_read.http_server;
                                let user_id = this.select_user_id.clone();
                                let task = Tokio::spawn(
                                    cx,
                                    create_private_chat(
                                        address,
                                        global_state_read.user_state.user_id,
                                        user_id,
                                    ),
                                );
                                cx.spawn(move |_, _: &mut AsyncApp| async move {
                                    let res = task.await;
                                    match res {
                                        Ok(Ok(())) => {}
                                        Ok(Err(e)) => println!("http error: {:?}", e),
                                        Err(e) => println!("tokio runtime error: {:?}", e),
                                    }
                                })
                                .detach();

                                global_state.update(cx, |this, cx| {
                                    cx.emit(EventBus::ChildrenChangeSelectIndex)
                                });
                            }),
                        ));
                        let menu = if can_add_friend {
                            menu.item(PopupMenuItem::new("添加好友").on_click(w.listener_for(
                                &entity_handle,
                                |this, _, _, cx| {
                                    request_friend(this, cx);
                                },
                            )))
                        } else {
                            menu
                        };
                        let menu = menu.item(
                            PopupMenuItem::new("查看资料")
                                .on_click(w.listener_for(&entity_handle, |this, _, _, cx| {})),
                        );
                        if !can_manage_roles {
                            return menu;
                        }
                        let selected_role = entity_handle.read_with(_cx, |entity, _| {
                            entity
                                .group_users
                                .iter()
                                .find(|member| member.id == entity.select_user_id)
                                .map(|member| member.user_type.clone())
                        });
                        match selected_role.as_deref() {
                            Some("owner") => menu,
                            Some("admin") => menu.item(PopupMenuItem::new("取消管理员").on_click(
                                w.listener_for(&entity_handle, |this, _, _, cx| {
                                    change_member_role(this, "member", cx);
                                }),
                            )),
                            _ => menu.item(PopupMenuItem::new("设为管理员").on_click(
                                w.listener_for(&entity_handle, |this, _, _, cx| {
                                    change_member_role(this, "admin", cx);
                                }),
                            )),
                        }
                    },
                )
                .into_any_element()
        }
    }
}
