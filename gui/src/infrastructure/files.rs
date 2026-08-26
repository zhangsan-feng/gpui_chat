use std::path::Path;
use std::process::Command;

use tokio::io::AsyncWriteExt;

pub async fn download_and_open(file_url: &str, temp_file_path: &Path) -> anyhow::Result<()> {
    let bytes = reqwest::get(file_url).await?.bytes().await?;
    let mut file = tokio::fs::File::create(temp_file_path).await?;
    file.write_all(&bytes).await?;

    let path = temp_file_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("path is not valid UTF-8"))?;
    #[cfg(target_os = "macos")]
    Command::new("open").arg(path).spawn()?;
    #[cfg(target_os = "linux")]
    Command::new("xdg-open").arg(path).spawn()?;
    #[cfg(target_os = "windows")]
    Command::new("cmd").args(["/C", "start", path]).spawn()?;
    Ok(())
}
