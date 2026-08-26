use crate::state::GlobalState;
use gpui::*;
use gpui_component::avatar::Avatar;
use gpui_component::label::Label;
use gpui_component::scroll::{Scrollbar, ScrollbarAxis, ScrollbarMode};
use gpui_component::{Icon, IconName, InteractiveElementExt, StyleSized, h_flex, v_flex};
use gpui_tokio::Tokio;
use std::path::PathBuf;
use std::process::Command;

use crate::domain::GroupHistory;
use crate::ui::{avatar_source, rgb_to_u32};
use log::info;
use tokio::io::AsyncWriteExt;

const MAX_MESSAGE_FILE_COUNT: usize = 4;

pub async fn download_and_open(
    file_url: &str,
    temp_file_path: &std::path::Path,
) -> anyhow::Result<()> {
    let bytes = reqwest::get(file_url).await?.bytes().await?;

    let mut file = tokio::fs::File::create(temp_file_path).await?;
    file.write_all(&bytes).await?;

    let s = temp_file_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("path not valid utf-8"))?;
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(s).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(s).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/C", "start", s]).spawn()?;
    }

    Ok(())
}

pub struct HistoryMessageEntity {
    pub scroll_handle: gpui::ListState,
    pub history_message: Vec<GroupHistory>,
}

impl HistoryMessageEntity {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        HistoryMessageEntity {
            scroll_handle: ListState::new(0, ListAlignment::Bottom, px(100.)),
            history_message: vec![],
        }
    }
}

impl Render for HistoryMessageEntity {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let global_state = cx.global::<GlobalState>().0.read(cx).clone();
        let history_message_scroll_handle = self.scroll_handle.clone();
        let entity_view = cx.entity().clone();

        if self.history_message.len() == 0 {
            return div().into_any_element();
        }
        let message = self.history_message.clone();

        h_flex()
            .mt_2()
            .size_full()
            .child(
                gpui::list(history_message_scroll_handle, move |index, window, app| {
                    let message = message[index].clone();

                    let formatted_message: String = message
                        .message
                        .chars()
                        .collect::<Vec<char>>()
                        .chunks(40)
                        .map(|chunk| chunk.iter().collect::<String>())
                        .collect::<Vec<String>>()
                        .join("\n");

                    let is_current_user =
                        global_state.user_state.user_id.clone() == message.send_user_id;

                    let element = h_flex().w_full().h_auto().py_2().px_3();

                    let message_image = message
                        .files
                        .iter()
                        .take(MAX_MESSAGE_FILE_COUNT)
                        .enumerate()
                        .map(|(index, file)| {
                            let id = format!("file-{}-{}", file, index);
                            let entity_view = entity_view.clone();
                            let file_path = file.to_string().clone();
                            let file_name = file_path
                                .split('/')
                                .last()
                                .filter(|name| !name.is_empty())
                                .unwrap_or("文件")
                                .to_string();
                            // let video_extensions = [".mp4", ".avi", ".mkv", ".mov", ".flv"];
                            let image_extensions =
                                [".png", ".jpg", ".gif", ".svg", ".jpeg", ".webp", ".bmp"];

                            div()
                                .py_1()
                                .id(id)
                                // .hover(|mut style|{
                                //     style.background = Some(rgb(rgb_to_u32(228, 228, 228)).into());
                                //     style
                                // })
                                .child(if image_extensions.iter().any(|&ext| file.contains(ext)) {
                                    img(file.to_string())
                                        .size_with(gpui_component::Size::Size(px(80.0)))
                                        .into_any_element()
                                } else {
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .p_2()
                                        .rounded(px(6.))
                                        .child(Icon::new(IconName::File).size(px(28.)))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .truncate()
                                                .child(Label::new(file_name.clone())),
                                        )
                                        .into_any_element()
                                })
                                .on_double_click(move |event, window, app| {
                                    let file_path = file_path.clone();
                                    entity_view.update(app, |this, cx| {
                                        let task = Tokio::spawn(cx, async move {
                                            let file_name =
                                                file_path.split('/').last().unwrap_or("");
                                            let mut temp_file_path: PathBuf = std::env::temp_dir();
                                            temp_file_path.push(file_name);

                                            match download_and_open(&*file_path, &*temp_file_path)
                                                .await
                                            {
                                                Ok(msg) => {}
                                                Err(err) => {
                                                    info!("error:{}", err)
                                                }
                                            }
                                        });
                                        cx.spawn(|_, _: &mut AsyncApp| async move {
                                            let _ = task.await;
                                        })
                                        .detach();
                                    })
                                })
                        });

                    let mut message_content = v_flex().w(relative(0.5)).min_w_0();
                    message_content = if is_current_user {
                        message_content.items_end()
                    } else {
                        message_content.items_start()
                    };
                    let message_content = message_content
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .truncate()
                                .child(if is_current_user {
                                    Label::new(message.send_username).text_right()
                                } else {
                                    Label::new(message.send_username)
                                }),
                        )
                        .child(
                            v_flex()
                                .items_start()
                                .min_w_0()
                                .max_w(relative(1.))
                                .bg(rgb(rgb_to_u32(228, 231, 235)))
                                .p_2()
                                .rounded(px(8.))
                                .child(Label::new(formatted_message))
                                .child(v_flex().gap_1().children(message_image)),
                        );

                    let avatar_source =
                        avatar_source(&message.send_user_avatar, &message.message_id);
                    let avatar = div().child(Avatar::new().src(avatar_source));

                    if is_current_user {
                        element
                            .justify_end()
                            .items_start()
                            .child(message_content)
                            .child(avatar)
                            .into_any_element()
                    } else {
                        element
                            .items_start()
                            .child(avatar)
                            .child(message_content)
                            .into_any_element()
                    }
                })
                .size_full()
                .p_2()
                .mb(px(20.))
                .into_any_element(),
            )
            .child(
                Scrollbar::vertical(&self.scroll_handle)
                    .mode(ScrollbarMode::Always)
                    .axis(ScrollbarAxis::Vertical),
            )
            .into_any_element()
    }
}
