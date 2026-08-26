use crate::domain::LoginResponseMsg;
use std::collections::HashSet;
use std::path::PathBuf;

use super::core;

pub struct SessionStore;

impl SessionStore {
    pub fn initialize(project_directory: PathBuf) -> anyhow::Result<()> {
        core::initialize(project_directory)
    }

    pub fn load_all() -> anyhow::Result<Vec<LoginResponseMsg>> {
        core::load_all()
    }

    pub fn save(session: &LoginResponseMsg) -> anyhow::Result<()> {
        core::save(session)
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

    pub fn remove(user_id: &str) -> anyhow::Result<()> {
        core::remove(user_id)
    }
}
