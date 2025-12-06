use anyhow::{Context, anyhow};

use crate::domain::intro_tool::ports::LocalAudioFetcher;

#[derive(Clone)]
pub struct Ffmpeg;

impl LocalAudioFetcher for Ffmpeg {
    async fn save_local_audio(&self, bytes: &[u8], name: &str) -> Result<String, anyhow::Error> {
        let temp_path = format!("./sounds/temp/{name}");
        let dest_path = format!("./sounds/{name}.mp3");

        // Write original file so its ready for codec conversion
        std::fs::write(&temp_path, bytes).context("failed to write temp file")?;
        let child = tokio::process::Command::new("ffmpeg")
            .args(["-i", &temp_path])
            .arg("-vn")
            .args(["-map", "0:a"])
            .arg(&dest_path)
            .spawn()
            .map_err(|err| anyhow!(err.to_string()))?
            .wait()
            .await
            .map_err(|err| anyhow!(err.to_string()))?;

        if !child.success() {
            return Err(anyhow!("ffmpeg terminated unsuccessfully"));
        }
        std::fs::remove_file(&temp_path).context("failed to remove temp file")?;

        Ok(format!("{name}.mp3"))
    }

    async fn delete_local_audio(&self, name: &str) -> Result<(), anyhow::Error> {
        std::fs::remove_file(format!("./sounds/{name}"))?;

        Ok(())
    }
}
