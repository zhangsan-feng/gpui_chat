use std::path::PathBuf;

use reqwest::multipart;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::infrastructure::http_request::HttpClient;

const MAX_MESSAGE_FILE_COUNT: usize = 4;

pub async fn send_message(
    address: String,
    _user_id: String,
    group_id: String,
    message: String,
    files: Vec<PathBuf>,
) -> anyhow::Result<()> {
    if files.len() > MAX_MESSAGE_FILE_COUNT {
        return Err(anyhow::anyhow!("最多发送4个文件"));
    }

    let mut form = multipart::Form::new()
        .text("group_id", group_id)
        .text("content", message);

    for path in files {
        let file = File::open(&path).await?;
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = reqwest::Body::wrap_stream(stream);
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".to_string());
        form = form.part("files", multipart::Part::stream(body).file_name(file_name));
    }

    HttpClient::new()
        .post_form(format!("{address}/api/v1/message/send"), form)
        .await?;
    Ok(())
}
