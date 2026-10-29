//! ffmpeg as a child process. Arguments go in as a list, never through a shell.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio_util::sync::CancellationToken;

use crate::error::{AppError, AppResult};
use crate::services::cancellable;

const SERVICE: &str = "ffmpeg";

/// First line of `ffmpeg -version`; doubles as the "is it installed" check.
pub async fn version(bin: &str) -> AppResult<String> {
    let out = command(bin).arg("-version").output().await.map_err(|e| spawn_err(bin, &e))?;
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text.lines().next().unwrap_or_default().to_owned())
}

/// Runs ffmpeg in `cwd`; cancelling kills the process.
pub async fn run(bin: &str, cwd: &Path, args: &[String], cancel: &CancellationToken) -> AppResult<()> {
    let child =
        command(bin).args(args).current_dir(cwd).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|e| spawn_err(bin, &e))?;
    let out = cancellable(cancel, async { Ok(child.wait_with_output().await?) }).await?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let tail: Vec<&str> = stderr.lines().rev().take(6).collect();
    let tail: Vec<&str> = tail.into_iter().rev().collect();
    Err(AppError::service(SERVICE, format!("ffmpeg dừng với lỗi ({}):\n{}", out.status, tail.join("\n"))))
}

fn command(bin: &str) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(locate(bin));
    cmd.stdin(Stdio::null()).kill_on_drop(true);
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW: no console window flashing up for every clip.
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

/// The plain name `ffmpeg` is looked up on PATH, then where winget and Homebrew install it: an app started from the
/// Start menu or Finder often has an older PATH than the terminal (winget adds to PATH only for new sessions, and
/// macOS apps never see Homebrew's folder). A full path from Settings is used as is.
fn locate(bin: &str) -> PathBuf {
    if bin != "ffmpeg" {
        return bin.into();
    }
    let exe = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    let path = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default();
    let mut candidates: Vec<PathBuf> = path.into_iter().map(|dir| dir.join(exe)).collect();
    candidates.extend(["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"].map(PathBuf::from));
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let winget = Path::new(&local).join("Microsoft").join("WinGet");
        candidates.push(winget.join("Links").join(exe));
        // Packages\Gyan.FFmpeg_<source>\ffmpeg-<version>-full_build\bin\ffmpeg.exe; the version changes on upgrade.
        let builds = std::fs::read_dir(winget.join("Packages"))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|p| p.file_name().to_string_lossy().starts_with("Gyan.FFmpeg"))
            .flat_map(|p| std::fs::read_dir(p.path()).into_iter().flatten().flatten())
            .map(|b| b.path().join("bin").join(exe));
        candidates.extend(builds);
    }
    candidates.into_iter().find(|p| p.is_file()).unwrap_or_else(|| bin.into())
}

fn spawn_err(bin: &str, e: &std::io::Error) -> AppError {
    if e.kind() == std::io::ErrorKind::NotFound {
        AppError::service(
            SERVICE,
            format!(
                "Không tìm thấy '{bin}'. Windows: chạy `winget install Gyan.FFmpeg` rồi mở lại terminal và app; Mac: `brew install ffmpeg`. Hoặc nhập đường dẫn ffmpeg trong Cài đặt."
            ),
        )
    } else {
        AppError::service(SERVICE, e.to_string())
    }
}
