use gpui::{
    App, AppContext, TitlebarOptions, WindowBounds, WindowDecorations, WindowOptions, px, size,
};
use gpui_component::Root;

use super::LoginView;

pub fn open(cx: &mut App) -> anyhow::Result<()> {
    let mut window_options = WindowOptions::default();
    window_options.window_bounds = Some(WindowBounds::centered(size(px(380.), px(450.)), cx));
    window_options.is_resizable = false;
    window_options.titlebar = Some(TitlebarOptions {
        appears_transparent: true,
        ..Default::default()
    });
    window_options.window_decorations = Some(WindowDecorations::Client);

    cx.open_window(window_options, |window, app| {
        gpui_component::init(app);
        let login_view = app.new(|cx| LoginView::new(window, cx));
        app.new(|cx| Root::new(login_view, window, cx))
    })
    .map(|_| ())
    .map_err(Into::into)
}
