use anyhow::Result;
use clap::Parser;
use rtrax::audio::command::Command;
use rtrax::audio::{self, FFT_RING_CAPACITY};
use rtrax::config::{Config, ThemeChoice};
use rtrax::playlist::Playlist;
use rtrax::state::SharedState;
use rtrax::ui::{restore_terminal_for_panic, App, Launch, PlayMode};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "rtrax", version, about = "TUI MOD/XM/IT/S3M/MTM module player")]
struct Cli {
    /// Module file(s) or a directory. Two or more files become an inline
    /// playlist; a single directory opens the browser there. With `--playlist`,
    /// a file/directory here switches to browse mode (the playlist is the save
    /// target for `a`).
    files: Vec<PathBuf>,

    /// Playlist file (.m3u). Alone, it plays as a queue (n/p walk it, Enter
    /// jumps). With a file/directory argument, it's the save target for `a`.
    #[arg(long, short = 'l', value_name = "FILE")]
    playlist: Option<PathBuf>,

    /// Override the theme set in config (e.g. neon-blue, c64, mono).
    #[arg(long, value_name = "THEME")]
    theme: Option<ThemeChoice>,

    /// Skip the config file and use built-in defaults.
    #[arg(long)]
    no_config: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    install_logger();
    install_panic_hook();

    let mut config = if cli.no_config {
        Config::default()
    } else {
        Config::load()
    };
    if let Some(theme) = cli.theme {
        config.theme = theme;
    }

    let launch = resolve_sources(cli.files, cli.playlist)?;

    let state = Arc::new(SharedState::new());
    let (fft_tx, fft_rx) = rtrb::RingBuffer::<f32>::new(FFT_RING_CAPACITY);
    let audio = audio::start(state.clone(), fft_tx)?;

    if let Some(path) = launch.initial_path.as_deref() {
        match audio::load_module(path) {
            Ok(loaded) => {
                audio::publish_loaded_metadata(&state, &loaded);
                audio.send(Command::Load(loaded.module));
            }
            Err(err) => tracing::warn!(?err, "failed to load initial module"),
        }
    }

    let app = App::new(state, audio, fft_rx, config, launch)?;
    app.run()
}

/// Decide the playback mode, the initial track, and the playlist/save target
/// from the CLI arguments.
///
/// - `--playlist <file>` alone → **queue mode**: play the playlist; `n`/`p`
///   walk it, Enter jumps, and the left panel is the queue.
/// - `--playlist <file>` + a file/directory → **browse mode**: browse the
///   given path and play from it; the playlist is purely the save target for
///   `a`.
/// - Two or more positional files → **queue mode** with an inline playlist.
/// - One positional file → **browse mode**, rooted at its folder, playing it.
/// - One positional directory → **browse mode**, rooted there, nothing playing.
/// - No arguments → **browse mode** at the default/working directory.
fn resolve_sources(files: Vec<PathBuf>, playlist_path: Option<PathBuf>) -> Result<Launch> {
    if let Some(pl_path) = playlist_path {
        if files.is_empty() {
            // Play the playlist as a queue.
            let queue = Playlist::load(pl_path.clone())?;
            let initial_path = queue.first().cloned();
            return Ok(Launch {
                initial_path,
                mode: PlayMode::Queue,
                queue: Some(queue),
                save_target: Some(pl_path),
                browse_root: None,
            });
        }
        // Build mode: browse the given path, append to the playlist with `a`.
        let (initial_path, browse_root) = browse_target(files);
        return Ok(Launch {
            initial_path,
            mode: PlayMode::Browse,
            queue: None,
            save_target: Some(pl_path),
            browse_root,
        });
    }

    match files.len() {
        0 => Ok(Launch {
            initial_path: None,
            mode: PlayMode::Browse,
            queue: None,
            save_target: None,
            browse_root: None,
        }),
        1 => {
            let (initial_path, browse_root) = browse_target(files);
            Ok(Launch {
                initial_path,
                mode: PlayMode::Browse,
                queue: None,
                save_target: None,
                browse_root,
            })
        }
        _ => {
            let queue = Playlist::from_files(files);
            let initial_path = queue.first().cloned();
            Ok(Launch {
                initial_path,
                mode: PlayMode::Queue,
                queue: Some(queue),
                save_target: None,
                browse_root: None,
            })
        }
    }
}

/// Resolve a positional argument into an initial track and a browser root. A
/// directory roots the browser there with nothing playing yet; a file plays
/// immediately and roots the browser at its parent folder.
fn browse_target(files: Vec<PathBuf>) -> (Option<PathBuf>, Option<PathBuf>) {
    match files.into_iter().next() {
        Some(p) if p.is_dir() => (None, Some(p)),
        Some(p) => {
            let root = p.parent().map(|parent| parent.to_path_buf());
            (Some(p), root)
        }
        None => (None, None),
    }
}

/// File-only logger. We MUST NOT write to stdout/stderr while ratatui owns the
/// terminal, since that corrupts the alternate-screen rendering.
fn install_logger() {
    let log_dir = dirs::cache_dir()
        .map(|p| p.join("rtrax"))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    if std::fs::create_dir_all(&log_dir).is_ok() {
        let file_appender = tracing_appender::rolling::daily(&log_dir, "rtrax.log");
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .with_writer(file_appender)
            .with_ansi(false)
            .try_init();
    }
}

/// Restore the terminal *before* the default panic handler prints, so the
/// panic message lands on a clean shell.
fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal_for_panic();
        prev(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn playlist_alone_plays_as_a_queue() {
        let dir = tempfile::tempdir().unwrap();
        let pl = dir.path().join("list.m3u");
        fs::write(&pl, "#EXTM3U\n/music/a.xm\n/music/b.xm\n").unwrap();

        let launch = resolve_sources(vec![], Some(pl.clone())).unwrap();
        assert_eq!(launch.mode, PlayMode::Queue);
        assert!(launch.queue.is_some());
        assert_eq!(launch.save_target, Some(pl));
        assert_eq!(launch.initial_path, Some(PathBuf::from("/music/a.xm")));
        assert!(launch.browse_root.is_none());
    }

    #[test]
    fn playlist_plus_directory_is_browse_mode_with_save_target() {
        let dir = tempfile::tempdir().unwrap();
        let pl = dir.path().join("list.m3u");
        fs::write(&pl, "#EXTM3U\n").unwrap();
        let browse = dir.path().join("songs");
        fs::create_dir(&browse).unwrap();

        let launch = resolve_sources(vec![browse.clone()], Some(pl.clone())).unwrap();
        assert_eq!(launch.mode, PlayMode::Browse);
        assert!(launch.queue.is_none()); // playlist is not the nav queue here
        assert_eq!(launch.save_target, Some(pl));
        assert_eq!(launch.browse_root, Some(browse));
        assert!(launch.initial_path.is_none());
    }

    #[test]
    fn single_file_is_browse_mode_rooted_at_its_folder() {
        let launch = resolve_sources(vec![PathBuf::from("/music/song.xm")], None).unwrap();
        assert_eq!(launch.mode, PlayMode::Browse);
        assert!(launch.queue.is_none());
        assert!(launch.save_target.is_none());
        assert_eq!(launch.initial_path, Some(PathBuf::from("/music/song.xm")));
        assert_eq!(launch.browse_root, Some(PathBuf::from("/music")));
    }

    #[test]
    fn single_directory_is_browse_mode_with_no_initial_track() {
        let dir = tempfile::tempdir().unwrap();
        let launch = resolve_sources(vec![dir.path().to_path_buf()], None).unwrap();
        assert_eq!(launch.mode, PlayMode::Browse);
        assert!(launch.initial_path.is_none());
        assert_eq!(launch.browse_root, Some(dir.path().to_path_buf()));
    }

    #[test]
    fn multiple_files_form_an_inline_queue() {
        let launch =
            resolve_sources(vec![PathBuf::from("/a.xm"), PathBuf::from("/b.xm")], None).unwrap();
        assert_eq!(launch.mode, PlayMode::Queue);
        assert_eq!(launch.queue.as_ref().map(|q| q.len()), Some(2));
        assert!(launch.save_target.is_none());
        assert_eq!(launch.initial_path, Some(PathBuf::from("/a.xm")));
    }

    #[test]
    fn no_arguments_is_browse_mode() {
        let launch = resolve_sources(vec![], None).unwrap();
        assert_eq!(launch.mode, PlayMode::Browse);
        assert!(launch.queue.is_none());
        assert!(launch.initial_path.is_none());
        assert!(launch.save_target.is_none());
        assert!(launch.browse_root.is_none());
    }
}
