//! Audio playback with speed and volume control.
//! Uses system commands (pw-play/play) for playback to avoid ALSA dependency.
//!
//! Playback can be cancelled by passing a stop signal (`Arc<AtomicBool>`).
//! When the signal is set to `true`, the child audio process is killed
//! and `play_audio` returns `Ok(())` immediately, allowing true pause/resume.

use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::error::GxxError;

/// Playback options
#[derive(Debug, Clone, Copy)]
pub struct PlaybackOptions {
    pub repeat: u32,
    pub speed: f32,  // 1.0 = normal, 0.7 = slow, 0.5 = very slow
    pub volume: f32, // 1.0 = normal, 0.6 = soft
}

impl Default for PlaybackOptions {
    fn default() -> Self {
        Self {
            repeat: 1,
            speed: 1.0,
            volume: 1.0,
        }
    }
}

/// Play audio samples with the given options using a system audio player.
/// This function blocks until playback completes.
///
/// If `stop_flag` is `Some`, it is checked periodically. When set to `true`,
/// the audio playback is killed immediately and the function returns early.
/// This enables pause/resume functionality.
pub fn play_audio(
    samples: &[f32],
    sample_rate: u32,
    options: PlaybackOptions,
    stop_flag: Option<&Arc<AtomicBool>>,
) -> Result<(), GxxError> {
    // A pending stop request takes precedence over validation and playback.
    if stop_flag.is_some_and(|flag| flag.load(Ordering::Acquire)) {
        return Ok(());
    }

    // Validate options
    if options.repeat == 0 || options.repeat > 10 {
        return Err(GxxError::msg("repeat must be between 1 and 10"));
    }
    if options.speed <= 0.0 || options.speed > 2.0 {
        return Err(GxxError::msg("speed must be between 0.0 and 2.0"));
    }
    if options.volume <= 0.0 || options.volume > 2.0 {
        return Err(GxxError::msg("volume must be between 0.0 and 2.0"));
    }

    // Process audio with speed and volume
    let processed =
        crate::tts::audio::process_audio(samples, options.speed, options.volume, sample_rate)?;

    // Write to WAV in memory
    let wav_bytes = crate::tts::audio::write_wav(&processed, sample_rate)?;

    // Try to find a working audio player. Each player has different stdin
    // conventions:
    //   pw-play   - reads WAV from stdin when given `-` as the file argument
    //   play (sox) - reads WAV from stdin but needs `-t wav` to know the format
    //   aplay     - reads raw PCM from stdin (needs -f cd -r <rate>)
    //   paplay   - reads WAV from stdin
    let players: &[(&str, &[&str])] = &[
        ("pw-play", &["-"]),
        ("play", &["-t", "wav", "-"]),
        ("paplay", &[]),
        ("aplay", &["-f", "cd", "-r", &sample_rate.to_string()]),
    ];

    let mut player_found = false;
    for (player, args) in players {
        // Check stop flag before each player attempt.
        if let Some(flag) = stop_flag {
            if flag.load(Ordering::Acquire) {
                return Ok(());
            }
        }

        if which(player).is_ok() {
            player_found = true;
            // Play the audio repeat times
            for _ in 0..options.repeat {
                // Check stop flag before each repeat.
                if let Some(flag) = stop_flag {
                    if flag.load(Ordering::Acquire) {
                        return Ok(());
                    }
                }

                let mut cmd = Command::new(player);
                for arg in *args {
                    cmd.arg(arg);
                }
                cmd.stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());

                let mut child = cmd
                    .spawn()
                    .map_err(|e| GxxError::msg(format!("Failed to start {}: {}", player, e)))?;

                // Write WAV data to stdin. A broken pipe here is not fatal: it
                // means the player exited early (e.g. user interrupted), so we
                // treat it as a successful completion of playback.
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    match stdin.write_all(&wav_bytes) {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
                        Err(e) => {
                            return Err(GxxError::msg(format!(
                                "Failed to write audio data: {}",
                                e
                            )));
                        }
                    }
                }
                // Drop stdin so the child sees EOF.
                drop(child.stdin.take());

                // Wait for playback to complete, checking the stop flag periodically.
                if !wait_for_child(&mut child, stop_flag)? {
                    // Child was killed due to stop signal.
                    eprintln!("[gxx playback] audio stopped by user");
                    return Ok(());
                }

                // Small delay between repeats
                thread::sleep(Duration::from_millis(100));
            }
            break;
        }
    }

    if !player_found {
        return Err(GxxError::msg(
            "No audio player found. Install one of: pw-play (pipewire), play (sox), paplay (pulseaudio), or aplay (alsa-utils)"
        ));
    }

    Ok(())
}

/// Wait for a child process to complete, checking a stop flag periodically.
/// Returns `true` if the child completed normally, `false` if it was killed
/// due to the stop flag being set.
fn wait_for_child(
    child: &mut Child,
    stop_flag: Option<&Arc<AtomicBool>>,
) -> Result<bool, GxxError> {
    loop {
        // Check if the child has exited.
        if let Some(_status) = child.try_wait()? {
            return Ok(true); // Child completed normally.
        }

        // Check stop flag.
        if let Some(flag) = stop_flag {
            if flag.load(Ordering::Acquire) {
                // Kill the child process.
                let _ = child.kill();
                let _ = child.wait();
                return Ok(false);
            }
        }

        // Brief sleep to avoid busy-waiting.
        thread::sleep(Duration::from_millis(20));
    }
}

/// Check if a command exists in PATH
fn which(cmd: &str) -> Result<std::path::PathBuf, GxxError> {
    let paths = std::env::var("PATH").unwrap_or_default();
    for path in paths.split(':') {
        let full_path = std::path::Path::new(path).join(cmd);
        if full_path.exists() && full_path.is_file() {
            return Ok(full_path);
        }
    }
    Err(GxxError::msg("not found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_which_finds_existing_command() {
        // `ls` is available on Unix
        #[cfg(unix)]
        {
            let result = which("ls");
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_which_not_found() {
        let result = which("this-command-does-not-exist-12345");
        assert!(result.is_err());
    }

    #[test]
    fn test_playback_options_default() {
        let opts = PlaybackOptions::default();
        assert_eq!(opts.repeat, 1);
        assert!((opts.speed - 1.0).abs() < 0.001);
        assert!((opts.volume - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_play_audio_invalid_repeat() {
        let samples = vec![0.0f32; 100];
        let opts = PlaybackOptions {
            repeat: 0,
            speed: 1.0,
            volume: 1.0,
        };
        assert!(play_audio(&samples, 22050, opts, None).is_err());
    }

    #[test]
    fn test_play_audio_invalid_repeat_high() {
        let samples = vec![0.0f32; 100];
        let opts = PlaybackOptions {
            repeat: 11,
            speed: 1.0,
            volume: 1.0,
        };
        assert!(play_audio(&samples, 22050, opts, None).is_err());
    }

    #[test]
    fn test_play_audio_invalid_speed() {
        let samples = vec![0.0f32; 100];
        let opts = PlaybackOptions {
            repeat: 1,
            speed: 0.0,
            volume: 1.0,
        };
        assert!(play_audio(&samples, 22050, opts, None).is_err());
    }

    #[test]
    fn test_play_audio_invalid_speed_high() {
        let samples = vec![0.0f32; 100];
        let opts = PlaybackOptions {
            repeat: 1,
            speed: 2.1,
            volume: 1.0,
        };
        assert!(play_audio(&samples, 22050, opts, None).is_err());
    }

    #[test]
    fn test_play_audio_invalid_volume() {
        let samples = vec![0.0f32; 100];
        let opts = PlaybackOptions {
            repeat: 1,
            speed: 1.0,
            volume: 0.0,
        };
        assert!(play_audio(&samples, 22050, opts, None).is_err());
    }

    #[test]
    fn test_play_audio_stop_flag_returns_ok() {
        // When the stop flag is set to true before playback starts,
        // play_audio should return Ok(()) immediately (no audio player
        // needed). This validates the cancellation path.
        let samples = vec![0.0f32; 100];
        let opts = PlaybackOptions {
            repeat: 1,
            speed: 1.0,
            volume: 1.0,
        };
        let stop_flag = Arc::new(AtomicBool::new(true));
        // Should return Ok without attempting to find an audio player.
        assert!(play_audio(&samples, 22050, opts, Some(&stop_flag)).is_ok());
    }

    #[test]
    fn test_play_audio_stop_flag_resets() {
        // After stopping, resetting the flag should allow normal validation.
        let opts = PlaybackOptions {
            repeat: 0, // invalid
            speed: 1.0,
            volume: 1.0,
        };
        let stop_flag = Arc::new(AtomicBool::new(true));
        // Even with invalid options, stop flag checked first returns Ok.
        assert!(play_audio(&[], 22050, opts, Some(&stop_flag)).is_ok());

        // After reset, invalid options should error.
        stop_flag.store(false, Ordering::Release);
        let opts = PlaybackOptions {
            repeat: 0,
            speed: 1.0,
            volume: 1.0,
        };
        assert!(play_audio(&[], 22050, opts, None).is_err());
    }
}
