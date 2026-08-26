use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use serde_json::json;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderValue, header::AUTHORIZATION};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};

pub async fn connect_forever(
    url: String,
    user_id: String,
    user_token: String,
    messages: mpsc::UnboundedSender<String>,
    connection_status: mpsc::UnboundedSender<bool>,
    mut stop: watch::Receiver<()>,
) {
    loop {
        if stop.has_changed().unwrap_or(true) {
            return;
        }

        info!("connecting to websocket server: {}", url);

        let mut request = match url.clone().into_client_request() {
            Ok(request) => request,
            Err(error) => {
                error!("failed to build websocket request: {}", error);
                let _ = connection_status.send(false);
                tokio::select! {
                    _ = stop.changed() => return,
                    _ = tokio::time::sleep(Duration::from_secs(10)) => {}
                }
                continue;
            }
        };
        let authorization = match HeaderValue::from_str(&format!("Bearer {user_token}")) {
            Ok(value) => value,
            Err(error) => {
                error!("failed to build websocket authorization header: {}", error);
                let _ = connection_status.send(false);
                tokio::select! {
                    _ = stop.changed() => return,
                    _ = tokio::time::sleep(Duration::from_secs(10)) => {}
                }
                continue;
            }
        };
        request.headers_mut().insert(AUTHORIZATION, authorization);

        let connection = tokio::select! {
            result = connect_async(request) => result,
            _ = stop.changed() => return,
        };

        match connection {
            Ok((websocket, _)) => {
                info!("websocket connected");
                let (mut writer, mut reader) = websocket.split();

                let payload = json!({
                    "id": user_id,
                    "token": user_token,
                });

                let auth_result = tokio::select! {
                    result = writer.send(Message::Text(Utf8Bytes::from(payload.to_string()))) => result,
                    _ = stop.changed() => return,
                };
                if let Err(error) = auth_result {
                    error!("failed to send websocket auth message: {}", error);
                    let _ = connection_status.send(false);
                    if stop.has_changed().unwrap_or(true) {
                        return;
                    }
                    tokio::select! {
                        _ = stop.changed() => return,
                        _ = tokio::time::sleep(Duration::from_secs(10)) => {}
                    }
                    continue;
                }
                let _ = connection_status.send(true);

                let mut interval = tokio::time::interval(Duration::from_secs(5));
                let mut stopped = false;
                loop {
                    tokio::select! {
                        _ = stop.changed() => {
                            stopped = true;
                            break;
                        }
                        _ = interval.tick() => {
                            if let Err(error) = writer
                                .send(Message::Ping(tokio_util::bytes::Bytes::new()))
                                .await
                            {
                                if !stop.has_changed().unwrap_or(true) {
                                    error!("websocket heartbeat failed: {}", error);
                                }
                                break;
                            }
                        }
                        message = reader.next() => {
                            match message {
                                Some(Ok(Message::Text(text))) => {
                                    if messages.send(text.to_string()).is_err() {
                                        info!("websocket message channel closed");
                                        return;
                                    }
                                }
                                Some(Ok(Message::Binary(data))) => {
                                    info!("received websocket binary message: {} bytes", data.len());
                                }
                                Some(Ok(Message::Close(frame))) => {
                                    info!("websocket closed by server: {:?}", frame);
                                    break;
                                }
                                Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {}
                                Some(Err(error)) => {
                                    error!("websocket read failed: {}", error);
                                    break;
                                }
                                None => break,
                                _ => {}
                            }
                        }
                    }
                }
                if stopped {
                    let _ = writer.send(Message::Close(None)).await;
                }
                let _ = connection_status.send(false);
                if stopped || stop.has_changed().unwrap_or(true) {
                    return;
                }
            }
            Err(error) => {
                error!("failed to connect websocket: {}", error);
                let _ = connection_status.send(false);
                if stop.has_changed().unwrap_or(true) {
                    return;
                }
            }
        }

        info!("websocket disconnected; retrying in 10 seconds");
        tokio::select! {
            _ = stop.changed() => return,
            _ = tokio::time::sleep(Duration::from_secs(10)) => {}
        }
    }
}
