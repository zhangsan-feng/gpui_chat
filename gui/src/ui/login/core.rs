use gpui::{AsyncApp, Context};
use gpui_tokio::Tokio;
use reqwest::StatusCode;

use super::{LoginView, Page};
use crate::application::{auth, session};
use crate::domain::LoginResponseMsg;
use crate::infrastructure::http_request::HttpResponseError;
use crate::state::GlobalState;
use crate::ui::component::file_picker::{self, FilePickerOptions};

const MINIMUM_PASSWORD_LENGTH: usize = 6;

pub fn select_saved_account(
    view: &mut LoginView,
    account_index: usize,
    cx: &mut Context<LoginView>,
) {
    if view.button_loading {
        return;
    }

    let Some(saved_session) = view.saved_sessions.get(account_index).cloned() else {
        return;
    };
    let login_name = saved_session.login_name.clone();
    if login_name.is_empty() {
        view.feedback = "该账号缺少登录名，请使用账号密码重新登录".to_string();
        cx.notify();
        return;
    }

    let now_timestamp = chrono::Utc::now().timestamp();
    if saved_session.needs_token_refresh(now_timestamp)
        && !saved_session.can_refresh_token(now_timestamp)
    {
        let _ = session::remove_saved_session(&login_name);
        view.saved_sessions
            .retain(|item| item.login_name != login_name);
        if view.saved_sessions.is_empty() {
            view.current_page = Page::Login;
        }
        view.feedback = "该账号登录状态已过期，请使用账号密码重新登录".to_string();
        cx.notify();
        return;
    }

    view.button_loading = true;
    view.feedback.clear();
    cx.notify();

    let global_state = cx.global::<GlobalState>().0.clone();
    let snapshot = global_state.read(cx).clone();
    let entity = cx.entity().clone();
    let mut async_cx = cx.to_async().clone();
    let needs_refresh = saved_session.needs_token_refresh(now_timestamp);
    let address = snapshot.http_server.clone();
    let task = Tokio::spawn(cx, async move {
        let auth_session = if needs_refresh {
            session::renew_saved_session(address.clone(), saved_session).await?
        } else {
            saved_session
        };
        let server_session = auth::validate_token(
            address,
            auth_session.user_id.clone(),
            auth_session.user_token.clone(),
        )
        .await?;

        Ok::<_, anyhow::Error>(auth_session.merge_refresh_result(server_session))
    });

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        match task.await {
            Ok(Ok(auth_session)) => match session::save_session(&auth_session) {
                Ok(()) => {
                    let _ = global_state.update(&mut async_cx, |state, _| {
                        state.set_user_session(auth_session.clone());
                    });
                    let _ = entity.update(&mut async_cx, |view, cx| {
                        upsert_saved_session(view, auth_session);
                        view.button_loading = false;
                        view.feedback.clear();
                        view.open_home_pending = true;
                        cx.notify();
                    });
                }
                Err(error) => update_error(&entity, &mut async_cx, error.to_string()),
            },
            Ok(Err(error)) => {
                update_saved_account_error(&entity, &mut async_cx, login_name.clone(), error)
            }
            Err(error) => update_saved_account_error(
                &entity,
                &mut async_cx,
                login_name.clone(),
                anyhow::anyhow!("保存账号登录任务失败: {error}"),
            ),
        }
    })
    .detach();
}

pub fn submit_login(view: &mut LoginView, cx: &mut Context<LoginView>) {
    if view.button_loading {
        return;
    }

    let login_name = view.login_name.read(cx).text().to_string();
    let login_name = login_name.trim().to_owned();
    let password = view.password.read(cx).text().to_string();

    if login_name.is_empty() || password.is_empty() {
        view.feedback = "请填写登录名和密码".to_string();
        cx.notify();
        return;
    }

    view.button_loading = true;
    view.feedback.clear();
    cx.notify();

    let global_state = cx.global::<GlobalState>().0.clone();
    let snapshot = global_state.read(cx).clone();
    let entity = cx.entity().clone();
    let mut async_cx = cx.to_async().clone();
    let task = Tokio::spawn(cx, auth::login(snapshot.http_server, login_name, password));

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        let result = task.await;

        match result {
            Ok(Ok(auth_session)) => match session::save_session(&auth_session) {
                Ok(()) => {
                    let _ = global_state.update(&mut async_cx, |state, _| {
                        state.set_user_session(auth_session.clone());
                    });
                    let _ = entity.update(&mut async_cx, |view, cx| {
                        upsert_saved_session(view, auth_session);
                        view.button_loading = false;
                        view.open_home_pending = true;
                        cx.notify();
                    });
                }
                Err(error) => update_error(&entity, &mut async_cx, error.to_string()),
            },
            Ok(Err(error)) => update_error(&entity, &mut async_cx, error.to_string()),
            Err(error) => update_error(&entity, &mut async_cx, error.to_string()),
        }
    })
    .detach();
}

pub fn submit_registration(view: &mut LoginView, cx: &mut Context<LoginView>) {
    if view.button_loading {
        return;
    }

    let login_name = view.login_name.read(cx).text().to_string();
    let login_name = login_name.trim().to_owned();
    let password = view.password.read(cx).text().to_string();
    let password_confirmation = view.password_confirmation.read(cx).text().to_string();

    if login_name.is_empty() || password.is_empty() || password_confirmation.is_empty() {
        view.feedback = "请填写登录名、密码和确认密码".to_string();
        cx.notify();
        return;
    }
    if password.len() < MINIMUM_PASSWORD_LENGTH {
        view.feedback = format!("密码至少需要 {MINIMUM_PASSWORD_LENGTH} 位");
        cx.notify();
        return;
    }
    if password != password_confirmation {
        view.feedback = "两次输入的密码不一致".to_string();
        cx.notify();
        return;
    }

    view.button_loading = true;
    view.feedback.clear();
    cx.notify();

    let global_state = cx.global::<GlobalState>().0.clone();
    let snapshot = global_state.read(cx).clone();
    let entity = cx.entity().clone();
    let mut async_cx = cx.to_async().clone();
    let task = Tokio::spawn(
        cx,
        auth::register(snapshot.http_server, login_name, password),
    );

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        let result = task.await;

        match result {
            Ok(Ok(auth_session)) => match session::save_session(&auth_session) {
                Ok(()) => {
                    let _ = global_state.update(&mut async_cx, |state, _| {
                        state.set_user_session(auth_session.clone());
                    });
                    let _ = entity.update(&mut async_cx, |view, cx| {
                        upsert_saved_session(view, auth_session);
                        view.button_loading = false;
                        view.open_home_pending = true;
                        cx.notify();
                    });
                }
                Err(error) => update_error(&entity, &mut async_cx, error.to_string()),
            },
            Ok(Err(error)) => update_error(&entity, &mut async_cx, error.to_string()),
            Err(error) => update_error(&entity, &mut async_cx, error.to_string()),
        }
    })
    .detach();
}

pub fn choose_avatar(view: &mut LoginView, cx: &mut Context<LoginView>) {
    if view.current_page != Page::Register || view.button_loading {
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
            if view.current_page != Page::Register {
                return;
            }
            view.avatar_path = Some(normalized_path.into());
            cx.notify();
        });
    })
    .detach();
}

pub fn show_login(view: &mut LoginView, cx: &mut Context<LoginView>) {
    view.current_page = Page::Login;
    view.feedback.clear();
    cx.notify();
}

pub fn show_account_selection(view: &mut LoginView, cx: &mut Context<LoginView>) {
    if view.saved_sessions.is_empty() {
        show_login(view, cx);
        return;
    }

    view.current_page = Page::AccountSelection;
    view.feedback.clear();
    cx.notify();
}

pub fn show_registration(view: &mut LoginView, cx: &mut Context<LoginView>) {
    view.current_page = Page::Register;
    view.feedback.clear();
    cx.notify();
}

fn upsert_saved_session(view: &mut LoginView, session: LoginResponseMsg) {
    view.saved_sessions
        .retain(|item| item.login_name != session.login_name);
    view.saved_sessions.push(session);
    view.saved_sessions
        .sort_by(|left, right| left.login_name.cmp(&right.login_name));
}

fn update_error(entity: &gpui::Entity<LoginView>, async_cx: &mut gpui::AsyncApp, message: String) {
    let _ = entity.update(async_cx, |view, cx| {
        view.button_loading = false;
        view.feedback = message;
        cx.notify();
    });
}

fn update_saved_account_error(
    entity: &gpui::Entity<LoginView>,
    async_cx: &mut gpui::AsyncApp,
    login_name: String,
    error: anyhow::Error,
) {
    let invalid_session = error
        .downcast_ref::<HttpResponseError>()
        .is_some_and(|response| {
            matches!(
                response.status(),
                StatusCode::UNAUTHORIZED | StatusCode::NOT_FOUND
            )
        });
    let message = if invalid_session {
        let _ = session::remove_saved_session(&login_name);
        "该账号登录状态已失效，请使用账号密码重新登录".to_string()
    } else if error
        .downcast_ref::<HttpResponseError>()
        .is_some_and(|response| response.status().is_server_error())
    {
        "服务端暂时不可用，请稍后重试".to_string()
    } else if error.downcast_ref::<HttpResponseError>().is_none() {
        "无法连接服务端，请确认服务端已启动".to_string()
    } else {
        error.to_string()
    };

    let _ = entity.update(async_cx, move |view, cx| {
        if invalid_session {
            view.saved_sessions
                .retain(|item| item.login_name != login_name);
            view.current_page = Page::Login;
        }
        view.button_loading = false;
        view.feedback = message;
        cx.notify();
    });
}
