use gpui::*;
use gpui_component::Root;

use super::AddFriendOrGroupWindow;
use crate::ui::component::window::window_center_options;

pub fn open<T>(parent_window: &mut Window, cx: &mut Context<T>) -> anyhow::Result<()>
where
    T: 'static,
{
    let window_size = size(px(480.), px(520.));

    cx.open_window(
        window_center_options(parent_window, window_size),
        |window, app| {
            let view = app.new(|cx| AddFriendOrGroupWindow::new(window, cx));
            app.new(|cx| Root::new(view, window, cx))
        },
    )
    .map_err(anyhow::Error::from)?;

    Ok(())
}
