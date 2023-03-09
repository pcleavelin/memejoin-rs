use crate::routes::Error;

pub(crate) async fn normalize(src: &str, dest: &str) -> Result<(), Error> {
    let child = tokio::process::Command::new("ffmpeg")
        .args(["-i", src])
        .arg("-vn")
        .args(["-map", "0:a"])
        .arg(dest)
        .spawn()
        .map_err(|err| Error::Ffmpeg(err.to_string()))?
        .wait()
        .await
        .map_err(|err| Error::Ffmpeg(err.to_string()))?;

    if !child.success() {
        return Err(Error::FfmpegTerminated);
    }

    Ok(())
}
