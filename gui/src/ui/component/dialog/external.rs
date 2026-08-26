#![allow(dead_code)]

use std::rc::Rc;

use gpui::{App, ClickEvent, ParentElement, SharedString, Window, px};
use gpui_component::WindowExt;
use gpui_component::button::Button;
use gpui_component::dialog::{AlertDialog, DialogFooter};

pub fn open_alert<F>(window: &mut Window, cx: &mut App, build: F)
where
    F: Fn(AlertDialog, &mut Window, &mut App) -> AlertDialog + 'static,
{
    window.open_alert_dialog(cx, build);
}

pub fn info(
    window: &mut Window,
    cx: &mut App,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
) {
    info_with_close(window, cx, title, description, |_, _, _| {});
}

pub fn info_with_close(
    window: &mut Window,
    cx: &mut App,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) {
    let title = title.into();
    let description = description.into();
    let on_close = Rc::new(on_close);

    open_alert(window, cx, move |dialog, _, _| {
        let on_ok = on_close.clone();
        let on_cancel = on_close.clone();
        dialog
            .width(px(360.))
            .title(title.clone())
            .description(description.clone())
            .footer(
                DialogFooter::new().child(Button::new("dialog-ok").label("知道了").on_click(
                    move |event, window, cx| {
                        on_ok(event, window, cx);
                        window.close_dialog(cx);
                    },
                )),
            )
            .on_cancel(move |event, window, cx| {
                on_cancel(event, window, cx);
                true
            })
    });
}

pub fn is_open(window: &mut Window, cx: &mut App) -> bool {
    window.has_active_dialog(cx)
}
