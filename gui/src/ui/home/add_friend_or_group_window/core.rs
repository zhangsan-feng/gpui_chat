use gpui::{AsyncApp, Context, Window};
use gpui_tokio::Tokio;

use super::AddFriendOrGroupWindow;
use crate::application::social;
use crate::state::GlobalState;

pub fn search(view: &mut AddFriendOrGroupWindow, cx: &mut Context<AddFriendOrGroupWindow>) {
    view.search_generation = view.search_generation.wrapping_add(1);
    let search_generation = view.search_generation;
    view.users.clear();
    view.groups.clear();

    let keyword = view
        .search_input
        .read(cx)
        .text()
        .to_string()
        .trim()
        .to_string();
    if keyword.is_empty() {
        view.feedback = "请输入用户名、用户 ID、群聊名称或群号".to_string();
        cx.notify();
        return;
    }

    let state = cx.global::<GlobalState>().0.read(cx).clone();
    let entity = cx.entity();
    let mut async_cx = cx.to_async();

    view.feedback = "正在搜索…".to_string();
    cx.notify();
    let task = Tokio::spawn(
        cx,
        social::search(state.http_server, state.user_state.user_id, keyword),
    );

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        let result = task.await;

        let _ = entity.update(&mut async_cx, |view, cx| {
            if view.search_generation != search_generation {
                return;
            }

            match result {
                Ok(Ok(result)) => {
                    view.users = result.users;
                    view.groups = result.groups;
                    view.feedback = if view.users.is_empty() && view.groups.is_empty() {
                        "没有找到匹配的好友或群聊".to_string()
                    } else {
                        String::new()
                    };
                    cx.notify();
                }
                Ok(Err(error)) => {
                    view.feedback = format!("搜索失败: {error}");
                    cx.notify();
                }
                Err(error) => {
                    view.feedback = format!("搜索任务失败: {error}");
                    cx.notify();
                }
            }
        });
    })
    .detach();
}

pub fn join_group(
    view: &mut AddFriendOrGroupWindow,
    group_id: String,
    window: &mut Window,
    cx: &mut Context<AddFriendOrGroupWindow>,
) {
    if view.pending_group_id.is_some() {
        return;
    }

    let state = cx.global::<GlobalState>().0.read(cx).clone();
    let entity = cx.entity();
    let window_handle = window.window_handle();
    let mut async_cx = cx.to_async();

    view.pending_group_id = Some(group_id.clone());
    view.feedback = "正在加入群聊…".to_string();
    cx.notify();
    let task = Tokio::spawn(
        cx,
        social::join_group(state.http_server, state.user_state.user_id, group_id),
    );

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        let result = task.await;

        match result {
            Ok(Ok(())) => {
                let _ = window_handle.update(&mut async_cx, |_, window, _| {
                    window.remove_window();
                });
            }
            Ok(Err(error)) => {
                let _ = entity.update(&mut async_cx, |view, cx| {
                    view.pending_group_id = None;
                    view.feedback = format!("加入群聊失败: {error}");
                    cx.notify();
                });
            }
            Err(error) => {
                let _ = entity.update(&mut async_cx, |view, cx| {
                    view.pending_group_id = None;
                    view.feedback = format!("加入群聊任务失败: {error}");
                    cx.notify();
                });
            }
        }
    })
    .detach();
}

pub fn add_friend(
    view: &mut AddFriendOrGroupWindow,
    friend_id: String,
    window: &mut Window,
    cx: &mut Context<AddFriendOrGroupWindow>,
) {
    if view.pending_friend_id.is_some() {
        return;
    }

    let state = cx.global::<GlobalState>().0.read(cx).clone();
    let entity = cx.entity();
    let window_handle = window.window_handle();
    let mut async_cx = cx.to_async();

    view.pending_friend_id = Some(friend_id.clone());
    view.feedback = "正在发送好友申请…".to_string();
    cx.notify();
    let task = Tokio::spawn(
        cx,
        social::add_friend(state.http_server, state.user_state.user_id, friend_id),
    );

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        let result = task.await;

        match result {
            Ok(Ok(())) => {
                let _ = window_handle.update(&mut async_cx, |_, window, _| {
                    window.remove_window();
                });
            }
            Ok(Err(error)) => {
                let _ = entity.update(&mut async_cx, |view, cx| {
                    view.pending_friend_id = None;
                    view.feedback = format!("发送好友申请失败: {error}");
                    cx.notify();
                });
            }
            Err(error) => {
                let _ = entity.update(&mut async_cx, |view, cx| {
                    view.pending_friend_id = None;
                    view.feedback = format!("发送好友申请任务失败: {error}");
                    cx.notify();
                });
            }
        }
    })
    .detach();
}
