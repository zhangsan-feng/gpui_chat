use gpui::{AsyncApp, Context};
use gpui_tokio::Tokio;
use log::info;
use tokio::sync::{mpsc, watch};
use tokio::time::Duration;

use super::{EventBus, State};
use crate::infrastructure::{http_request, session_store::SessionStore, websocket};

impl State {
    pub fn new(_: &mut Context<Self>) -> Self {
        Self {
            http_server: String::from("http://127.0.0.1:34332"),
            ws_server: String::from("ws://127.0.0.1:34332"),
            is_login: false,
            user_state: Default::default(),
            server_connected: false,
            ws_stop: None,
            ws_generation: 0,
        }
    }

    pub fn set_user_session(&mut self, session: super::LoginResponseMsg) {
        self.user_state = session;
        self.is_login = self.user_state.is_authenticated();
    }

    pub fn clear_user_session(&mut self) {
        self.stop_ws();
        self.user_state = Default::default();
        self.is_login = false;
    }

    pub fn init_ws(&mut self, cx: &mut Context<Self>) {
        self.stop_ws();
        if !self.is_login {
            return;
        }

        info!("initializing websocket connection");
        let (ws_stop, stop_receiver) = watch::channel(());
        self.ws_stop = Some(ws_stop);
        let ws_generation = self.ws_generation;

        let entity = cx.entity().clone();
        let async_cx = cx.to_async().clone();
        let (messages, mut received_messages) = mpsc::unbounded_channel::<String>();
        let (connection_status, mut received_connection_status) = mpsc::unbounded_channel::<bool>();
        let message_entity = entity.clone();
        let mut message_async_cx = async_cx.clone();
        let status_entity = entity.clone();
        let mut status_async_cx = async_cx.clone();

        cx.spawn(|_, _: &mut AsyncApp| async move {
            while let Some(message) = received_messages.recv().await {
                let _ = message_entity.update(&mut message_async_cx, |_, cx| {
                    cx.emit(EventBus::WebSocketText(message));
                });
            }
        })
        .detach();

        cx.spawn(move |_, _: &mut AsyncApp| async move {
            while let Some(connected) = received_connection_status.recv().await {
                let _ = status_entity.update(&mut status_async_cx, |state, cx| {
                    if state.ws_generation != ws_generation {
                        return;
                    }
                    state.server_connected = connected;
                    cx.emit(EventBus::WebSocketConnectionChanged);
                });
            }
        })
        .detach();

        let expiration_entity = entity.clone();
        let expired_login_name = self.user_state.login_name.clone();
        let mut expiration_async_cx = cx.to_async().clone();
        let mut expiration_stop = stop_receiver.clone();
        let expiration_task = Tokio::spawn(cx, async move {
            loop {
                tokio::select! {
                    _ = expiration_stop.changed() => return false,
                    _ = tokio::time::sleep(Duration::from_millis(250)) => {}
                }
                if http_request::take_auth_expired() {
                    return true;
                }
            }
        });

        cx.spawn(|_, _: &mut AsyncApp| async move {
            if !matches!(expiration_task.await, Ok(true)) {
                return;
            }

            if !expired_login_name.is_empty() {
                let _ = SessionStore::remove(&expired_login_name);
            }
            http_request::clear_auth_tokens();
            let _ = expiration_entity.update(&mut expiration_async_cx, |state, cx| {
                state.clear_user_session();
                state.server_connected = false;
                cx.emit(EventBus::SessionExpired);
            });
        })
        .detach();

        let url = format!("{}/ws", self.ws_server);
        let user_id = self.user_state.user_id.clone();
        let user_token = self.user_state.user_token.clone();

        Tokio::spawn(
            cx,
            websocket::connect_forever(
                url,
                user_id,
                user_token,
                messages,
                connection_status,
                stop_receiver,
            ),
        )
        .detach();
    }

    pub fn stop_ws(&mut self) {
        self.ws_generation = self.ws_generation.wrapping_add(1);
        if let Some(ws_stop) = self.ws_stop.take() {
            let _ = ws_stop.send(());
        }
        self.server_connected = false;
    }
}
