use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::popover::Popover;
use gpui_component::separator::Separator;
use gpui_component::{Icon, IconName, h_flex, v_flex};

use super::core;
use super::{HistoryMessagePanelResizeHandle, LeftPanelResizeHandle, MessagePage};
use crate::state::GlobalState;
use crate::ui::rgb_to_u32;

pub fn render(
    page: &mut MessagePage,
    window: &mut Window,
    cx: &mut Context<MessagePage>,
) -> impl IntoElement {
    let current_user_id = cx
        .global::<GlobalState>()
        .0
        .read(cx)
        .user_state
        .user_id
        .clone();
    let selected_group = page.message_group.get(page.select_index).cloned();
    let title = selected_group
        .as_ref()
        .map(|group| {
            if group.group_type != "private_chat" {
                return group.name.clone();
            }

            group
                .members
                .iter()
                .find(|member| member.id.as_str() != current_user_id.as_str())
                .map(|member| {
                    if member.name.is_empty() {
                        group.name.clone()
                    } else {
                        member.name.clone()
                    }
                })
                .unwrap_or_else(|| group.name.clone())
        })
        .unwrap_or_default();
    let settings_button = selected_group
        .filter(|group| group.group_type == "group")
        .map(|group| {
            let settings_entity = page.group_settings_entity.clone();
            let settings_entity_for_open = settings_entity.clone();
            let current_user_id_for_open = current_user_id.clone();
            let group_for_open = group.clone();
            let trigger = Button::new("message-page-group-settings")
                .text()
                .compact()
                .px_2()
                .child(Icon::new(IconName::Settings2).size(px(18.)));

            Popover::new("message-page-group-settings-popover")
                .anchor(Anchor::TopRight)
                .trigger(trigger)
                .on_open_change(move |open, window, app| {
                    if *open {
                        settings_entity_for_open.update(app, |panel, cx| {
                            panel.sync_group(
                                &group_for_open,
                                &current_user_id_for_open,
                                window,
                                cx,
                            );
                        });
                    }
                })
                .content(move |_, _, _| settings_entity.clone())
                .into_any_element()
        })
        .unwrap_or_else(|| div().into_any_element());

    h_flex()
        .size_full()
        .child(page.left_sidebar(window, cx))
        .child(
            div()
                .id("left_resize_handle")
                .w(px(3.0))
                .h_full()
                .bg(rgb(rgb_to_u32(235, 235, 235)))
                .cursor_col_resize()
                .active(|style| style.bg(rgb(0x007acc)))
                .on_drag(LeftPanelResizeHandle, |handle, _, _, app| {
                    app.new(|_| handle.clone())
                })
                .on_drag_move(cx.listener(|page, event, _, _| {
                    core::left_panel_handle_resize(page, event);
                })),
        )
        .child(
            v_flex()
                .flex_1()
                .size_full()
                .bg(rgb(rgb_to_u32(255, 255, 255)))
                .child(
                    h_flex()
                        .h(px(60.))
                        .w_full()
                        .p_4()
                        .items_center()
                        .child(title)
                        .child(div().flex_1())
                        .child(settings_button),
                )
                .child(Separator::horizontal().w_full())
                .child(if page.message_group.is_empty() {
                    div().flex_1().into_any_element()
                } else {
                    h_flex()
                        .flex_1()
                        .size_full()
                        .child(
                            v_flex()
                                .flex_1()
                                .size_full()
                                .child(div().flex_1().child(page.history_message_entity.clone()))
                                .child(
                                    div()
                                        .id("history_message_resize_handle")
                                        .h(px(3.0))
                                        .w_full()
                                        .bg(rgb(rgb_to_u32(235, 235, 235)))
                                        .cursor_row_resize()
                                        .active(|style| style.bg(rgb(0x007acc)))
                                        .on_drag(
                                            HistoryMessagePanelResizeHandle,
                                            |handle, _, _, app| app.new(|_| handle.clone()),
                                        )
                                        .on_drag_move(cx.listener(|page, event, window, cx| {
                                            core::history_message_handle_resize(
                                                page, event, window, cx,
                                            );
                                        })),
                                )
                                .child(page.sned_message_entity.clone()),
                        )
                        .child(Separator::vertical().h_full())
                        .child(page.group_members_entity.clone())
                        .into_any_element()
                }),
        )
}
