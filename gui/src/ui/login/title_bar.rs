use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, Rgba, Role,
    StatefulInteractiveElement, Styled, Window, WindowControlArea, div, px, rgb,
};
use gpui_component::h_flex;

pub struct CustomTitleBar;

impl CustomTitleBar {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self
    }

    fn render_close_button(&self, cx: &Context<Self>) -> impl IntoElement {
        let hover_color: Rgba = rgb(0xdbeafe);

        div()
            .id("login-titlebar-close")
            .role(Role::Button)
            .aria_label("关闭")
            .size(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(0xf7fbff))
            .text_color(rgb(0x0369a1))
            .hover(move |style| style.bg(hover_color))
            .on_click(cx.listener(|_, _, window, _| window.remove_window()))
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(gpui::FontWeight::NORMAL)
                    .text_color(rgb(0x1e3a5f))
                    .child("×"),
            )
    }
}

impl Render for CustomTitleBar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .id("login-titlebar")
            .w_full()
            .h(px(38.))
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(rgb(0xbae6fd))
            .bg(rgb(0xf7fbff))
            .child(
                h_flex()
                    .id("login-titlebar-drag")
                    .h_full()
                    .flex_1()
                    .items_center()
                    .px_4()
                    .window_control_area(WindowControlArea::Drag)
                    .child(div()),
            )
            .child(
                h_flex()
                    .h_full()
                    .items_center()
                    .border_l_1()
                    .border_color(rgb(0xbae6fd))
                    .gap_0()
                    .child(self.render_close_button(cx)),
            )
    }
}
