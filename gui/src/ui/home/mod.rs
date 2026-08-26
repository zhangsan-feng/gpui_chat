use gpui::{
    App, AppContext, Context, Entity, IntoElement, Render, SharedString, TitlebarOptions, Window,
    WindowBounds, WindowDecorations, WindowOptions, px, size,
};
use gpui_component::Root;

mod add_friend_or_group_window;
mod friend_page;
mod message_page;
mod notification;
mod user_profile;

mod core;
mod title_bar;
mod ui;

use self::friend_page::FriendPage;
use self::message_page::MessagePage;
use self::notification::NotificationCenter;
use self::title_bar::CustomTitleBar;

pub struct HomeView {
    select_page: i32,
    title_bar: Entity<CustomTitleBar>,
    notification_center: Entity<NotificationCenter>,
    message_page: Entity<MessagePage>,
    friend_page: Entity<FriendPage>,
    session_expired: bool,
}

impl HomeView {
    pub fn open(cx: &mut App) -> anyhow::Result<()> {
        let mut window_options = WindowOptions::default();
        let window_size = size(px(1200.), px(700.));
        window_options.window_bounds = Some(WindowBounds::centered(window_size, cx));
        window_options.window_min_size = Some(window_size);
        window_options.titlebar = Some(TitlebarOptions {
            title: Some(SharedString::from("")),
            appears_transparent: true,
            ..Default::default()
        });
        window_options.window_decorations = Some(WindowDecorations::Client);

        cx.open_window(window_options, |window, app| {
            let state_handle = app.global::<crate::state::GlobalState>().0.clone();
            state_handle
                .clone()
                .update(app, |state, cx| state.init_ws(cx));
            gpui_component::init(app);

            let login_name = state_handle.read(app).user_state.login_name.clone();
            let notification_center = app.new(|cx| NotificationCenter::new(login_name.clone(), cx));
            let mut home_view = Self {
                select_page: 0,
                title_bar: app.new(|cx| {
                    CustomTitleBar::new(
                        window,
                        cx,
                        state_handle.clone(),
                        notification_center.clone(),
                    )
                }),
                notification_center,
                message_page: app.new(|cx| MessagePage::new(cx, window)),
                friend_page: app.new(|cx| FriendPage::new(cx, window)),
                session_expired: false,
            };

            let view = app.new(|cx| {
                core::load_component_data(&home_view, cx);
                core::subscribe_websocket(&mut home_view, cx);
                home_view
            });

            app.new(|cx| Root::new(view, window, cx))
        })
        .map(|_| ())
        .map_err(Into::into)
    }
}

impl Render for HomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ui::render(self, window, cx)
    }
}
