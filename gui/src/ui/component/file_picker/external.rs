use std::path::PathBuf;

use futures_util::future::BoxFuture;
use gpui::{Context, PathPromptOptions};

use super::FilePickerOptions;

pub fn choose_files<T: 'static>(
    cx: &mut Context<T>,
    options: FilePickerOptions,
) -> BoxFuture<'static, Option<Vec<PathBuf>>> {
    let receiver = cx.prompt_for_paths(PathPromptOptions::from(options));
    Box::pin(async move { receiver.await.ok().and_then(|result| result.ok()).flatten() })
}
