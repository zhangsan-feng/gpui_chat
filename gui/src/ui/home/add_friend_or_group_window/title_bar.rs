use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, Rgba,
    StatefulInteractiveElement, Styled, Window, WindowControlArea, div, px, rgb,
};
use gpui_component::h_flex;

pub struct CustomTitleBar;

impl CustomTitleBar {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self
    }

    fn render_close_button(&self, cx: &Context<Self>) -> impl IntoElement {
        let hover_color: Rgba = rgb(0xf4cad7);

        div()
            .id("add-friend-window-titlebar-close")
            .size(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(0x49425c))
            .hover(move |style| style.bg(hover_color))
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .id("add-friend-window-titlebar")
            .w_full()
            .h(px(38.))
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(rgb(0xe7dceb))
            .bg(rgb(0xfaf7fc))
            .child(
                h_flex()
                    .id("add-friend-window-titlebar-drag")
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
                            .child("添加好友/群聊"),
                    ),
            )
            .child(
                h_flex()
                    .h_full()
                    .items_center()
                    .border_l_1()
                    .border_color(rgb(0xe7dceb))
                    .child(self.render_close_button(cx)),
            )
    }
}
