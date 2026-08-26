use crate::domain::LoginResponseMsg;
use std::collections::HashSet;

use super::core;

pub fn load_saved_sessions() -> anyhow::Result<Vec<LoginResponseMsg>> {
    core::load_saved_sessions()
}

pub fn save_session(session: &LoginResponseMsg) -> anyhow::Result<()> {
    core::save_session(session)
}

pub fn load_notification_read_ids(login_name: &str) -> anyhow::Result<HashSet<String>> {
    core::load_notification_read_ids(login_name)
}

pub fn mark_notifications_read(
    login_name: &str,
    notification_ids: &[String],
) -> anyhow::Result<()> {
    core::mark_notifications_read(login_name, notification_ids)
}

pub fn mark_notification_unread(login_name: &str, notification_id: &str) -> anyhow::Result<()> {
    core::mark_notification_unread(login_name, notification_id)
}

pub fn remove_saved_session(user_id: &str) -> anyhow::Result<()> {
    core::remove_saved_session(user_id)
}

pub fn clear_active_session() {
    core::clear_active_session()
}

pub async fn renew_saved_session(
    address: String,
    current_session: LoginResponseMsg,
) -> anyhow::Result<LoginResponseMsg> {
    core::renew_saved_session(address, current_session).await
}
