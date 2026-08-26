use std::path::PathBuf;

use crate::application::group::create_group_chat;
use crate::state::GlobalState;
use crate::ui::component::file_picker::{self, FilePickerOptions};
use gpui::*;
use gpui_component::avatar::Avatar;
use gpui_component::button::Button;
use gpui_component::form::field;
use gpui_component::input::{Input, InputState};
use gpui_component::*;
use gpui_tokio::Tokio;

use super::title_bar::CustomTitleBar;

pub struct CreateGroupChatWindow {
    title_bar: Entity<CustomTitleBar>,
    group_name: Entity<InputState>,
    avatar_path: Option<PathBuf>,
}

impl CreateGroupChatWindow {
    pub fn new(cx: &mut Context<Self>, window: &mut Window) -> Self {
        CreateGroupChatWindow {
            title_bar: cx.new(|cx| CustomTitleBar::new(window, cx)),
            group_name: cx.new(|cx| InputState::new(window, cx)),
            avatar_path: Default::default(),
        }
    }
}

impl Render for CreateGroupChatWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(self.title_bar.clone())
            .child(
                v_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .py_6()
                    .child(
                        div()
                            .flex_none()
                            .size(px(120.))
                            .rounded_full()
                            .border_2()
                            .border_color(rgb(0x9999AF))
                            .bg(rgb(0x333333))
                            .cursor_pointer()
                            .overflow_hidden()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    let mut async_cx = cx.to_async();
                                    let entity_view = cx.entity();

                                    cx.spawn(|_, _: &mut AsyncApp| async move {
                                        let paths = entity_view.update(&mut async_cx, |_, cx| {
                                            file_picker::choose_files(
                                                cx,
                                                FilePickerOptions::single_file(),
                                            )
                                        });

                                        if let Some(path) =
                                            paths.await.and_then(|paths| paths.first().cloned())
                                        {
                                            entity_view.update(&mut async_cx, |this, cx| {
                                                this.avatar_path = Some(path);
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .detach();
                                }),
                            )
                            .child(if let Some(path) = &self.avatar_path {
                                div().size_full().rounded_full().overflow_hidden().child(
                                    Avatar::new()
                                        .src(path.clone())
                                        .size_full()
                                        .with_size(gpui_component::Size::Size(px(120.0))),
                                )
                            } else {
                                div().size_full().rounded_full().bg(rgb(0x333333))
                            }),
                    )
                    .child(
                        field()
                            .label("群聊名称")
                            .items_center()
                            .child(Input::new(&self.group_name).w(px(240.)).h(px(50.))),
                    )
                    .child(
                        Button::new("create_group_chat")
                            .label("创建")
                            .mt_2()
                            .justify_center()
                            .on_click(cx.listener(|this, _, window, cx| {
                                let global_state = cx.global::<GlobalState>().0.read(cx).clone();
                                let group_name = this.group_name.read(cx).text().to_string();
                                let avatar_path = this.avatar_path.clone();
                                let address = global_state.http_server;
                                let window_handler = window.window_handle();
                                let mut cx_async = cx.to_async().clone();
                                let task = Tokio::spawn(
                                    cx,
                                    create_group_chat(
                                        address,
                                        global_state.user_state.user_id,
                                        group_name,
                                        avatar_path,
                                    ),
                                );

                                cx.spawn(move |_, _: &mut AsyncApp| async move {
                                    let res = task.await;
                                    match res {
                                        Ok(Ok(true)) => {
                                            let _ = window_handler.update(
                                                &mut cx_async,
                                                |_, window, _| {
                                                    window.remove_window();
                                                },
                                            );
                                        }
                                        Ok(Ok(false)) => {
                                            log::error!("group chat creation was rejected")
                                        }
                                        Ok(Err(error)) => {
                                            log::error!("group chat creation failed: {}", error)
                                        }
                                        Err(error) => {
                                            log::error!("group chat task failed: {}", error)
                                        }
                                    }
                                })
                                .detach()
                            })),
                    ),
            )
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
    }
}
