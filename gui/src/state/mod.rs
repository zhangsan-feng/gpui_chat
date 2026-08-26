use gpui::{App, AppContext, Entity, EventEmitter, Global};
use serde::Deserialize;
use tokio::sync::watch;

use crate::domain::LoginResponseMsg;

mod core;

#[derive(Clone)]
pub struct State {
    pub http_server: String,
    pub ws_server: String,
    pub is_login: bool,
    pub user_state: LoginResponseMsg,
    pub server_connected: bool,
    ws_stop: Option<watch::Sender<()>>,
    ws_generation: u64,
}

#[derive(Clone, Deserialize)]
pub enum EventBus {
    WebSocketText(String),
    WebSocketConnectionChanged,
    ChildrenChangeSelectIndex,
    OpenConversation(String),
    OpenPrivateChat(String),
    RemoveConversation(String),
    SessionExpired,
}

impl EventEmitter<EventBus> for State {}

pub struct GlobalState(pub Entity<State>);

impl Global for GlobalState {}

pub fn new_state(cx: &mut App) {
    let state_entity = cx.new(|cx| State::new(cx));
    cx.set_global(GlobalState(state_entity));
}
