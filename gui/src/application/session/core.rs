use crate::application::auth;
use std::collections::HashSet;

use crate::domain::LoginResponseMsg;
use crate::infrastructure::http_request;
use crate::infrastructure::session_store::SessionStore;

pub fn load_saved_sessions() -> anyhow::Result<Vec<LoginResponseMsg>> {
    SessionStore::load_all()
}

pub fn save_session(session: &LoginResponseMsg) -> anyhow::Result<()> {
    let result = SessionStore::save(session);
    if result.is_ok() {
        http_request::set_auth_tokens(&session.user_token, &session.refresh_token);
    }
    result
}

pub fn load_notification_read_ids(login_name: &str) -> anyhow::Result<HashSet<String>> {
    SessionStore::load_notification_read_ids(login_name)
}

pub fn mark_notifications_read(
    login_name: &str,
    notification_ids: &[String],
) -> anyhow::Result<()> {
    SessionStore::mark_notifications_read(login_name, notification_ids)
}

pub fn mark_notification_unread(login_name: &str, notification_id: &str) -> anyhow::Result<()> {
    SessionStore::mark_notification_unread(login_name, notification_id)
}

pub fn remove_saved_session(user_id: &str) -> anyhow::Result<()> {
    SessionStore::remove(user_id)
}

pub fn clear_active_session() {
    http_request::clear_auth_tokens();
}

pub async fn renew_saved_session(
    address: String,
    current_session: LoginResponseMsg,
) -> anyhow::Result<LoginResponseMsg> {
    let refreshed = auth::refresh_token(address, current_session.refresh_token.clone()).await?;
    let session = current_session.merge_refresh_result(refreshed);
    save_session(&session)?;
    Ok(session)
}
