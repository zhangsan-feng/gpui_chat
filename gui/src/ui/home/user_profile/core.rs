use gpui::{AsyncApp, ClipboardItem, Context, Window};
use gpui_tokio::Tokio;

use crate::application::{profile, session};
use crate::ui::component::file_picker::{self, FilePickerOptions};
use crate::ui::component::notification;
use crate::ui::home::user_profile::UserProfileEditor;

const MINIMUM_PASSWORD_LENGTH: usize = 6;

pub fn copy_user_id(
    view: &mut UserProfileEditor,
    window: &mut Window,
    cx: &mut Context<UserProfileEditor>,
) {
    let user_id = view.state.read(cx).user_state.user_id.clone();
    if user_id.is_empty() {
        return;
    }

    cx.write_to_clipboard(ClipboardItem::new_string(user_id));
    notification::success(window, cx, "已经复制");
}

pub fn choose_avatar(view: &mut UserProfileEditor, cx: &mut Context<UserProfileEditor>) {
    if view.button_loading {
        return;
    }

    let entity = cx.entity().clone();
    let mut async_cx = cx.to_async().clone();
    cx.spawn(move |_, _: &mut AsyncApp| async move {
        let paths = entity.update(&mut async_cx, |_, cx| {
            file_picker::choose_files(cx, FilePickerOptions::single_file())
        });

        let Some(path) = paths.await.and_then(|paths| paths.into_iter().next()) else {
            return;
        };

        let normalized_path = path.to_string_lossy().replace('\\', "/");
        let _ = entity.update(&mut async_cx, |view, cx| {
            if !view.button_loading {
                view.avatar_path = Some(normalized_path);
                cx.notify();
            }
        });
    })
    .detach();
}

pub fn submit(view: &mut UserProfileEditor, cx: &mut Context<UserProfileEditor>) {
    let username = view.username.read(cx).value().to_string();
    let username = username.trim().to_owned();
    let current_password = view.current_password.read(cx).value().to_string();
    let new_password = view.new_password.read(cx).value().to_string();
    let password_confirmation = view.password_confirmation.read(cx).value().to_string();

    if username.is_empty() {
        view.feedback = "用户名不能为空".to_string();
        cx.notify();
        return;
    }

    if new_password.is_empty() && !password_confirmation.is_empty() {
        view.feedback = "请输入新密码".to_string();
        cx.notify();
        return;
    }

    if !new_password.is_empty() && current_password.is_empty() {
        view.feedback = "修改密码时请填写当前密码".to_string();
        cx.notify();
        return;
    }

    if !new_password.is_empty() && new_password.len() < MINIMUM_PASSWORD_LENGTH {
        view.feedback = format!("新密码至少需要 {MINIMUM_PASSWORD_LENGTH} 位");
        cx.notify();
        return;
    }

    if new_password != password_confirmation {
        view.feedback = "两次输入的新密码不一致".to_string();
        cx.notify();
        return;
    }

    view.button_loading = true;
    view.feedback.clear();
    cx.notify();

    let state = view.state.clone();
    let current_session = state.read(cx).user_state.clone();
    let address = state.read(cx).http_server.clone();
    let avatar_path = view.avatar_path.clone();
    let entity = cx.entity().clone();
    let mut async_cx = cx.to_async().clone();
    let task = Tokio::spawn(
        cx,
        profile::update_profile(
            address,
            username,
            current_session.login_name.clone(),
            current_password,
            new_password,
            avatar_path,
        ),
    );

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        let result = task.await;

        match result {
            Ok(Ok(mut auth_session)) => {
                if auth_session.login_name.is_empty() {
                    auth_session.login_name = current_session.login_name.clone();
                }

                match session::save_session(&auth_session) {
                    Ok(()) => {
                        let avatar_url = auth_session.user_avatar.clone();
                        let next_session = auth_session.clone();
                        let _ = state.update(&mut async_cx, |state, cx| {
                            state.set_user_session(next_session);
                            cx.notify();
                        });
                        let _ = entity.update(&mut async_cx, |view, cx| {
                            view.button_loading = false;
                            view.avatar_path = (!avatar_url.is_empty()).then_some(avatar_url);
                            view.reset_password_inputs = true;
                            cx.notify();
                        });
                    }
                    Err(error) => {
                        log::error!("failed to save updated user session: {}", error);
                        update_error(&entity, &mut async_cx, error.to_string());
                    }
                }
            }
            Ok(Err(error)) => {
                log::error!("failed to update user profile: {}", error);
                update_error(&entity, &mut async_cx, error.to_string());
            }
            Err(error) => {
                log::error!("profile update task failed: {}", error);
                update_error(&entity, &mut async_cx, error.to_string());
            }
        }
    })
    .detach();
}

pub fn logout(
    view: &mut UserProfileEditor,
    window: &mut Window,
    cx: &mut Context<UserProfileEditor>,
) {
    session::clear_active_session();
    let _ = view.state.update(cx, |state, _| {
        state.clear_user_session();
        state.server_connected = false;
    });
    match crate::ui::login::open(cx) {
        Ok(()) => window.remove_window(),
        Err(error) => log::error!("failed to open login window during logout: {}", error),
    }
}

fn update_error(
    entity: &gpui::Entity<UserProfileEditor>,
    async_cx: &mut gpui::AsyncApp,
    message: String,
) {
    let _ = entity.update(async_cx, |view, cx| {
        view.button_loading = false;
        view.feedback = message;
        cx.notify();
    });
}
