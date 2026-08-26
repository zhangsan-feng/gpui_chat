use gpui::prelude::FluentBuilder;
use gpui::{
    Anchor, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Pixels,
    Render, Rgba, StatefulInteractiveElement, Styled, Subscription, Window, WindowControlArea, div,
    px, rgb,
};
use gpui_component::avatar::Avatar;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::popover::Popover;
use gpui_component::{Sizable, Size, h_flex};

use crate::state::State;
use crate::ui::home::notification::NotificationCenter;
use crate::ui::home::user_profile::UserProfileEditor;

pub struct CustomTitleBar {
    state: Entity<State>,
    profile_editor: Entity<UserProfileEditor>,
    notification_center: Entity<NotificationCenter>,
    _state_subscription: Subscription,
    _notification_subscription: Subscription,
}

impl CustomTitleBar {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        state: Entity<State>,
        notification_center: Entity<NotificationCenter>,
    ) -> Self {
        let profile_editor =
            cx.new(|profile_cx| UserProfileEditor::new(window, profile_cx, state.clone()));
        let state_subscription = cx.observe(&state, |_, _, cx| cx.notify());
        let notification_subscription = cx.observe(&notification_center, |_, _, cx| cx.notify());

        Self {
            state,
            profile_editor,
            notification_center,
            _state_subscription: state_subscription,
            _notification_subscription: notification_subscription,
        }
    }

    fn render_user_menu(&self, cx: &Context<Self>) -> impl IntoElement {
        let user_avatar = self.state.read(cx).user_state.user_avatar.clone();
        let profile_editor = self.profile_editor.clone();
        let trigger = Button::new("home-titlebar-user")
            .text()
            .compact()
            .h_full()
            .px_2()
            .bg(rgb(0xfaf7fc))
            .child(Self::render_avatar(user_avatar, px(28.)));

        Popover::new("home-user-popover")
            .anchor(Anchor::TopLeft)
            .appearance(false)
            .trigger(trigger)
            .content(move |_, _, _| profile_editor.clone())
    }

    fn render_avatar(source: String, size: Pixels) -> Avatar {
        Avatar::new()
            .when(!source.is_empty(), |this| this.src(source))
            .with_size(Size::Size(size))
    }

    fn render_connection_status(&self, cx: &Context<Self>) -> impl IntoElement {
        let connected = self.state.read(cx).server_connected;
        div()
            .text_size(px(12.))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(if connected {
                rgb(0x34745b)
            } else {
                rgb(0xd14b4b)
            })
            .child(if connected { "已连接" } else { "未连接" })
    }

    fn render_notification_menu(&self, cx: &Context<Self>) -> impl IntoElement {
        let notification_center = self.notification_center.clone();
        let notification_center_for_open = notification_center.clone();
        let unread_count = notification_center.read(cx).unread_count();
        let trigger = Button::new("home-titlebar-notification")
            .text()
            .compact()
            .h_full()
            .px_2()
            .child(
                div()
                    .relative()
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(16.))
                    .child("🔔")
                    .when(unread_count > 0, |this| {
                        this.child(
                            div()
                                .absolute()
                                .top(px(-2.))
                                .right(px(-4.))
                                .min_w(px(15.))
                                .h(px(15.))
                                .px_1()
                                .rounded_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(rgb(0xef4444))
                                .text_color(rgb(0xffffff))
                                .text_size(px(9.))
                                .child(unread_count.to_string()),
                        )
                    }),
            );

        Popover::new("home-notification-popover")
            .anchor(Anchor::TopRight)
            .trigger(trigger)
            .on_open_change(move |open, _, app| {
                notification_center_for_open.update(app, |center, cx| {
                    center.set_popover_open(*open, cx);
                });
            })
            .content(move |_, _, _| notification_center.clone())
    }

    fn render_window_button(
        &self,
        id: &'static str,
        label: &'static str,
        control: WindowControlArea,
        hover_color: Rgba,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        div()
            .id(id)
            .size(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(0x49425c))
            .hover(move |style| style.bg(hover_color))
            .window_control_area(control)
            .when(cfg!(target_os = "linux"), move |this| {
                this.on_click(cx.listener(move |_, _, window, _| match control {
                    WindowControlArea::Min => window.minimize_window(),
                    WindowControlArea::Max => window.zoom_window(),
                    _ => {}
                }))
            })
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(gpui::FontWeight::NORMAL)
                    .child(label),
            )
    }

    fn render_close_button(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .id("home-titlebar-close")
            .size(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(0x49425c))
            .hover(|style| style.bg(rgb(0xf4cad7)))
            .on_click(cx.listener(|_, _, window, _| window.remove_window()))
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(gpui::FontWeight::NORMAL)
                    .child("×"),
            )
    }
}

impl Render for CustomTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let minimize = self.render_window_button(
            "home-titlebar-minimize",
            "−",
            WindowControlArea::Min,
            rgb(0xe8d8f0),
            cx,
        );
        let maximize = self.render_window_button(
            "home-titlebar-maximize",
            if window.is_maximized() { "❐" } else { "□" },
            WindowControlArea::Max,
            rgb(0xe8d8f0),
            cx,
        );
        let close = self.render_close_button(cx);

        h_flex()
            .id("home-titlebar")
            .w_full()
            .h(px(38.))
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(rgb(0xe7dceb))
            .bg(rgb(0xfaf7fc))
            .child(self.render_user_menu(cx))
            .child(
                h_flex()
                    .id("home-titlebar-drag")
                    .h_full()
                    .flex_1()
                    .items_center()
                    .px_4()
                    .window_control_area(WindowControlArea::Drag)
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(0x49425c))
                            .child(self.render_connection_status(cx)),
                    ),
            )
            .child(
                h_flex()
                    .h_full()
                    .items_center()
                    .border_l_1()
                    .border_color(rgb(0xe7dceb))
                    .gap_0()
                    .children(vec![
                        self.render_notification_menu(cx).into_any_element(),
                        minimize.into_any_element(),
                        maximize.into_any_element(),
                        close.into_any_element(),
                    ]),
            )
    }
}
