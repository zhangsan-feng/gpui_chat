use gpui::{AsyncApp, Context, Window};
use gpui_tokio::Tokio;

use super::GroupSettingsPanel;
use crate::application::group;
use crate::domain::MessageGroup;
use crate::state::GlobalState;
use crate::ui::component::file_picker::{self, FilePickerOptions};

pub fn sync_group(
    panel: &mut GroupSettingsPanel,
    group: &MessageGroup,
    current_user_id: &str,
    window: &mut Window,
    cx: &mut Context<GroupSettingsPanel>,
) {
    let can_edit = group.group_type == "group"
        && group
            .members
            .iter()
            .any(|member| member.id == current_user_id && member.user_type == "owner");
    let current_name = panel.group_name.read(cx).value().to_string();
    if panel.group_id == group.id
        && current_name == group.name
        && panel.group_avatar == group.avatar
        && panel.allow_join == group.allow_join
        && panel.can_edit == can_edit
    {
        return;
    }

    panel.group_id = group.id.clone();
    panel.group_avatar = group.avatar.clone();
    panel.avatar_path = None;
    panel.allow_join = group.allow_join;
    panel.can_edit = can_edit;
    panel.saving = false;
    panel.feedback.clear();
    panel.group_name.update(cx, |input, cx| {
        input.set_value(group.name.clone(), window, cx)
    });
    cx.notify();
}

pub fn choose_avatar(panel: &mut GroupSettingsPanel, cx: &mut Context<GroupSettingsPanel>) {
    if !panel.can_edit || panel.saving {
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
        let _ = entity.update(&mut async_cx, |panel, cx| {
            if panel.can_edit && !panel.saving {
                panel.avatar_path = Some(path);
                cx.notify();
            }
        });
    })
    .detach();
}

pub fn toggle_allow_join(
    panel: &mut GroupSettingsPanel,
    allow_join: &bool,
    cx: &mut Context<GroupSettingsPanel>,
) {
    if panel.can_edit && !panel.saving {
        panel.allow_join = *allow_join;
        cx.notify();
    }
}

pub fn submit(panel: &mut GroupSettingsPanel, cx: &mut Context<GroupSettingsPanel>) {
    if !panel.can_edit || panel.saving || panel.group_id.is_empty() {
        return;
    }

    let group_name = panel.group_name.read(cx).value().trim().to_owned();
    if group_name.is_empty() {
        panel.feedback = "群聊名称不能为空".to_string();
        cx.notify();
        return;
    }

    let state = cx.global::<GlobalState>().0.clone();
    let snapshot = state.read(cx).clone();
    let address = snapshot.http_server;
    let group_id = panel.group_id.clone();
    let avatar_path = panel.avatar_path.clone();
    let allow_join = panel.allow_join;
    let entity = cx.entity().clone();
    let mut async_cx = cx.to_async().clone();
    panel.saving = true;
    panel.feedback.clear();
    cx.notify();

    let task = Tokio::spawn(
        cx,
        group::update_group(address, group_id, group_name, avatar_path, allow_join),
    );
    cx.spawn(move |_, _: &mut AsyncApp| async move {
        match task.await {
            Ok(Ok(())) => {
                let _ = entity.update(&mut async_cx, |panel, cx| {
                    panel.saving = false;
                    panel.feedback = "已保存".to_string();
                    cx.notify();
                });
            }
            Ok(Err(error)) => set_error(&entity, &mut async_cx, error.to_string()),
            Err(error) => set_error(&entity, &mut async_cx, error.to_string()),
        }
    })
    .detach();
}

fn set_error(entity: &gpui::Entity<GroupSettingsPanel>, cx: &mut gpui::AsyncApp, message: String) {
    let _ = entity.update(cx, |panel, cx| {
        panel.saving = false;
        panel.feedback = message;
        cx.notify();
    });
}
