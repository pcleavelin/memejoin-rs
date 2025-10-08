use anyhow::{anyhow, Context};

use crate::domain::intro_tool::ports::RemoteAudioFetcher;

#[derive(Clone)]
pub struct Ytdlp;

impl RemoteAudioFetcher for Ytdlp {
    async fn fetch_remote_audio(&self, url: &str, name: &str) -> Result<String, anyhow::Error> {
        let file_name = format!("sounds/{name}");

        let child = tokio::process::Command::new("yt-dlp")
            .arg(url)
            .args(["-o", &file_name])
            .args(["-x", "--audio-format", "mp3"])
            .spawn()
            .context("failed to spawn yt-dlp process")?
            .wait()
            .await
            .context("yt-dlp process failed")?;

        if !child.success() {
            return Err(anyhow!("yt-dlp terminated unsuccessfully"));
        }

        Ok(format!("{file_name}.mp3"))
    }
}
