use std::collections::HashSet;
use std::path::PathBuf;

use crate::state::GlobalState;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::input::{Textarea, TextareaState};
use gpui_component::label::Label;
use gpui_component::{h_flex, v_flex};
use log::{error, info};

use crate::application::message::send_message;
use crate::ui::component::file_picker::{self, FilePickerOptions};
use crate::ui::rgb_to_u32;
use gpui_tokio::Tokio;

const MAX_MESSAGE_FILE_COUNT: usize = 4;

pub struct SendMessageEntity {
    text_input: Entity<TextareaState>,
    pick_send_files: Vec<PathBuf>,
    pub group_id: String,
    pub panel_height: f32,
    err_msg: String,
    sending: bool,
}

impl SendMessageEntity {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        SendMessageEntity {
            text_input: cx.new(|cx| TextareaState::new(window, cx)),
            pick_send_files: Vec::new(),
            group_id: String::new(),
            panel_height: 160.0,
            err_msg: String::new(),
            sending: false,
        }
    }

    pub fn pick_files(&self, cx: &mut Context<Self>) {
        let mut async_cx = cx.to_async();
        let entity_view = cx.entity();

        cx.spawn(|_, _: &mut AsyncApp| async move {
            let paths = entity_view.update(&mut async_cx, |_, cx| {
                file_picker::choose_files(cx, FilePickerOptions::multiple_files())
            });

            if let Some(path_vec) = paths.await {
                info!("{:?}", path_vec);

                entity_view.update(&mut async_cx, move |this, cx| {
                    let existing_set: HashSet<_> = this.pick_send_files.iter().collect();
                    let mut new_files: Vec<_> = path_vec
                        .into_iter()
                        .filter(|p| !existing_set.contains(p))
                        .collect();
                    let remaining =
                        MAX_MESSAGE_FILE_COUNT.saturating_sub(this.pick_send_files.len());
                    if new_files.len() > remaining {
                        new_files.truncate(remaining);
                        this.err_msg = format!("最多选择{}个文件", MAX_MESSAGE_FILE_COUNT);
                    }
                    this.pick_send_files.extend(new_files);
                    cx.notify();
                });

                // if let Some(first_path) = path_vec.first() {
                //     let path_str = first_path.to_string_lossy().to_string();
                //     entity_view.update(&mut async_cx, |this, cx| {
                //         // update_this.avatar_path = Some(path_str.replace("\\", "/").into());
                //     });
                // }
            }
        })
        .detach();
    }
}

impl Render for SendMessageEntity {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .h(px(self.panel_height))
            .w_full()
            .py_2()
            .px_2()
            .child(
                h_flex()
                    .ml_1()
                    .child(
                        div()
                            .id("sned_message_component-folder-id")
                            .child(img("icon/icons8-folder-96.png").size(px(24.)))
                            .p_1()
                            .rounded(px(4.))
                            .hover(|mut style| {
                                style.background = Some(rgb(rgb_to_u32(235, 235, 235)).into());
                                style
                            })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, event, window, cx| this.pick_files(cx)),
                            ),
                    )
                    .child(
                        div()
                            .id("sned_message_component-folder-id")
                            .child(img("icon/icons8-scissors-50.png").size(px(24.)))
                            .p_1()
                            .rounded(px(4.))
                            .hover(|mut style| {
                                style.background = Some(rgb(rgb_to_u32(235, 235, 235)).into());
                                style
                            })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, event, window, cx| {}),
                            ),
                    )
                    .children(
                        self.pick_send_files
                            .iter()
                            .enumerate()
                            .map(|(index, path)| {
                                div()
                                    .mx_1()
                                    .relative()
                                    .size(px(25.0))
                                    .child(img(path.clone()).size(px(24.0)))
                                    .child(
                                        Button::new(("remove_img", index.clone()))
                                            .label("×")
                                            .p_0()
                                            .size(px(12.0))
                                            .w(px(12.0))
                                            .h(px(12.0))
                                            .absolute()
                                            .top(px(-6.0))
                                            .right(px(-6.0))
                                            .bg(gpui::red())
                                            .text_color(gpui::white())
                                            .rounded_full()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .on_click(cx.listener(move |this, _, _, _| {
                                                if !this.err_msg.is_empty() {
                                                    this.err_msg = String::new();
                                                }
                                                this.pick_send_files.remove(index);
                                            })),
                                    )
                            }),
                    )
                    .child(Label::new(self.err_msg.to_string()))
                    .child(div().flex_grow_1())
                    .child(
                        div()
                            .child(img("icon/icons8-history-96.png").size(px(24.)))
                            .id("sned_message_component-history-id")
                            .p_1()
                            .rounded(px(4.))
                            .hover(|mut style| {
                                style.background = Some(rgb(rgb_to_u32(235, 235, 235)).into());
                                style
                            }),
                    ),
            )
            .child(
                Textarea::new(&self.text_input)
                    .bordered(false)
                    .h(relative(1.)),
            )
            .child(
                h_flex().items_end().justify_end().child(
                    Button::new("send_message")
                        .label(if self.sending {
                            "发送中…"
                        } else {
                            "发送"
                        })
                        .bg(rgb(rgb_to_u32(0, 141, 235)))
                        .w(px(120.))
                        .on_click(cx.listener(|this, event, window, cx| {
                            if this.sending {
                                return;
                            }

                            let global_state = cx.global::<GlobalState>().0.clone();
                            let user_id = global_state.read(cx).clone().user_state.user_id;
                            let send_group_id = this.group_id.clone();

                            let address = global_state.read(cx).http_server.clone();
                            let msg = this.text_input.read(cx).text().to_string();
                            let pick_files = this.pick_send_files.clone();
                            let entity_view = cx.entity();
                            let window_handler = window.window_handle();
                            let mut async_cx = cx.to_async().clone();

                            info!("{}", msg);
                            let task = Tokio::spawn(
                                cx,
                                send_message(address, user_id, send_group_id, msg, pick_files),
                            );

                            this.sending = true;
                            this.err_msg.clear();
                            cx.notify();

                            cx.spawn(move |_, _: &mut AsyncApp| async move {
                                let result = task.await;

                                let _ = window_handler.update(&mut async_cx, |_, window, cx| {
                                    let _ = entity_view.update(cx, |this, cx| {
                                        this.sending = false;
                                        match result {
                                            Ok(Ok(())) => {
                                                this.text_input.update(cx, |state, cx| {
                                                    state.set_value("", window, cx);
                                                });
                                                this.pick_send_files.clear();
                                                this.err_msg.clear();
                                            }
                                            Ok(Err(error)) => {
                                                error!("发送消息失败: {error}");
                                                this.err_msg = format!("发送失败: {error}");
                                            }
                                            Err(error) => {
                                                error!("发送消息任务失败: {error}");
                                                this.err_msg = format!("发送任务失败: {error}");
                                            }
                                        }
                                        cx.notify();
                                    });
                                });
                            })
                            .detach();
                        })),
                ),
            )
            .into_any_element()
    }
}
