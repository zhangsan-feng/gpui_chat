use gpui::{AsyncApp, Context};
use gpui_tokio::Tokio;
use log::error;

use super::HomeView;
use crate::application::home::load_component_data as load_home_data;
use crate::application::notification as notification_application;
use crate::domain::WsMsgEvent;
use crate::state::{EventBus, GlobalState};

pub fn subscribe_websocket(home: &mut HomeView, cx: &mut Context<HomeView>) {
    let state_handle = cx.global::<GlobalState>().0.clone();

    cx.subscribe(&state_handle, |this, _, event: &EventBus, cx| match event {
        EventBus::WebSocketText(text) => match serde_json::from_str::<WsMsgEvent>(text) {
            Ok(event) => {
                this.notification_center.update(cx, |center, cx| {
                    center.update_component_data(event.clone(), cx);
                });
                this.message_page.update(cx, |page, cx| {
                    page.update_component_data(event.clone(), cx);
                });
                this.friend_page.update(cx, |page, cx| {
                    page.update_component_data(event, cx);
                });
            }
            Err(error) => {
                error!("failed to parse websocket event: {}", error);
            }
        },
        EventBus::WebSocketConnectionChanged => {
            cx.notify();
        }
        EventBus::ChildrenChangeSelectIndex => {
            this.select_page = 0;
            cx.notify();
        }
        EventBus::OpenConversation(group_id) => {
            this.select_page = 0;
            this.message_page.update(cx, |page, cx| {
                page.open_conversation(group_id.clone(), cx);
            });
            cx.notify();
        }
        EventBus::OpenPrivateChat(user_id) => {
            this.select_page = 0;
            this.message_page.update(cx, |page, cx| {
                page.open_private_chat(user_id.clone(), cx);
            });
            cx.notify();
        }
        EventBus::RemoveConversation(group_id) => {
            this.message_page.update(cx, |page, cx| {
                page.remove_conversation(group_id.clone(), cx);
            });
        }
        EventBus::SessionExpired => {
            this.session_expired = true;
            cx.notify();
        }
    })
    .detach();
}

pub fn load_component_data(home: &HomeView, cx: &mut Context<HomeView>) {
    let state = cx.global::<GlobalState>().0.clone().read(cx).clone();
    let user_id = state.user_state.user_id;
    let address = state.http_server;
    let notification_address = address.clone();
    let entity = cx.entity().clone();
    let mut async_cx = cx.to_async().clone();
    let task = Tokio::spawn(cx, load_home_data(address, user_id));
    let notification_entity = home.notification_center.clone();
    let mut notification_async_cx = cx.to_async().clone();
    let notification_task = Tokio::spawn(cx, notification_application::list(notification_address));

    cx.spawn(move |_, _: &mut AsyncApp| async move {
        let response = task.await;

        match response {
            Ok(Ok(data)) => {
                entity.update(&mut async_cx, |home, cx| {
                    home.message_page.update(cx, |page, cx| {
                        page.init_component_data(data.message_groups.clone(), cx);
                    });
                    home.friend_page.update(cx, |page, cx| {
                        page.init_component_data(
                            data.friends.clone(),
                            data.message_groups.clone(),
                            cx,
                        );
                    });
                });
            }
            Ok(Err(error)) => error!("failed to load initial component data: {}", error),
            Err(error) => error!("initial component task failed: {}", error),
        }

        match notification_task.await {
            Ok(Ok(notifications)) => {
                let _ = notification_entity.update(&mut notification_async_cx, |center, cx| {
                    center.init_component_data(notifications, cx);
                });
            }
            Ok(Err(error)) => error!("failed to load notifications: {}", error),
            Err(error) => error!("notification task failed: {}", error),
        }
    })
    .detach();
}
