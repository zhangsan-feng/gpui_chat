use gpui::*;
use gpui_component::separator::Separator;
use gpui_component::{Root, h_flex, v_flex};
use log::error;

use super::HomeView;
use crate::ui::rgb_to_u32;

pub fn render(
    home: &mut HomeView,
    window: &mut Window,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    if home.session_expired {
        home.session_expired = false;
        let state_handle = cx.global::<crate::state::GlobalState>().0.clone();
        let login_name = state_handle.read(cx).user_state.login_name.clone();
        if !login_name.is_empty() {
            if let Err(error) = crate::application::session::remove_saved_session(&login_name) {
                error!("failed to clear expired session: {}", error);
            }
        }
        state_handle.update(cx, |state, _| state.clear_user_session());
        match crate::ui::login::open(cx) {
            Ok(()) => window.remove_window(),
            Err(error) => error!("failed to open login window after token expiry: {}", error),
        }
    }

    let icons = ["icon/message_icon.png", "icon/user_icon.png"];

    v_flex()
        .size_full()
        .bg(rgb(rgb_to_u32(248, 249, 251)))
        .child(home.title_bar.clone())
        .child(
            h_flex()
                .items_start()
                .flex_1()
                .child(
                    v_flex()
                        .items_center()
                        .w(px(60.))
                        .h_full()
                        .py_2()
                        .children(icons.iter().enumerate().map(|(index, icon_path)| {
                            let selected = home.select_page == index as i32;
                            div()
                                .id(("home-navigation", index))
                                .rounded_2xl()
                                .flex()
                                .w(px(40.))
                                .h(px(40.))
                                .items_center()
                                .justify_center()
                                .m_1()
                                .bg(if selected {
                                    rgb(rgb_to_u32(0, 153, 255))
                                } else {
                                    rgb(rgb_to_u32(245, 245, 245))
                                })
                                .hover(|mut style| {
                                    style.background = Some(rgb(rgb_to_u32(225, 235, 245)).into());
                                    style
                                })
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.select_page = index as i32;
                                    cx.notify();
                                }))
                                .child(
                                    img((*icon_path).to_string())
                                        .size(px(25.))
                                        .object_fit(ObjectFit::Cover),
                                )
                        }))
                        .child(div().flex_1()),
                )
                .child(Separator::vertical().h_full())
                .child(match home.select_page {
                    0 => home.message_page.clone().into_any_element(),
                    1 => home.friend_page.clone().into_any_element(),
                    _ => div().size_full().into_any_element(),
                }),
        )
        .children(Root::render_dialog_layer(window, cx))
        .children(Root::render_notification_layer(window, cx))
        .children(Root::render_sheet_layer(window, cx))
}
