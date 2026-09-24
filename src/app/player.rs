use super::MyApp;
use crate::wwise::*;
use std::path::PathBuf;
// use std::time::{Duration, Instant};
use std::time::Instant;
use rodio::Source;
use ww2ogg::validate;

impl MyApp {

    // opens default audio device for app lifetime
    pub(super) fn ensure_audio_stream(&mut self) -> bool {
        if self.audio_sink.is_some() {
            return true;
        }
        match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(mut sink) => {
                sink.log_on_drop(false);
                self.audio_sink = Some(sink);
                true
            }
            Err(e) => {
                self.log_and_status(&format!("Failed to open audio device: {}", e));
                false
            }
        }
    }

    // plays a file from beginning
    pub(super) fn start_player(&mut self, path: PathBuf) {
        self.start_player_at(path, std::time::Duration::ZERO);
    }

    pub(super) fn start_player_at(&mut self, path: PathBuf, start_at: std::time::Duration) {
        if !self.ensure_audio_stream() {
            return;
        }
        let cached: Option<Vec<u8>> = if self.player_source.as_ref() == Some(&path) {
            self.player_bytes.clone()
        } else {
            None
        };
        let (bytes, source_duration): (Vec<u8>, std::time::Duration) = match cached {
            Some(b) => (b, self.player_duration),
            None => {
                match path.extension().and_then(|e| e.to_str()) {
                    Some("wem") => {
                        let raw = match std::fs::read(&path) {
                            Ok(b) => b,
                            Err(e) => {
                                self.log_and_status(&format!("Failed to read WEM: {}", e));
                                return;
                            }
                        };
                        let ogg = match try_with_codebooks(&raw, false) {
                            Ok(d) if validate(&d).is_ok() => d,
                            _ => match try_with_codebooks(&raw, true) {
                                Ok(d) => d,
                                Err(e) => {
                                    self.log_and_status(&format!("WEM decode failed: {}", e));
                                    return;
                                }
                            }
                        };
                        let duration = wem_sample_rate(&path)
                            .zip(ogg_total_samples(&ogg))
                            .filter(|(_, s)| *s > 0)
                            .map(|(sr, s)| std::time::Duration::from_secs_f64(s as f64 / sr as f64))
                            .unwrap_or(std::time::Duration::ZERO);
                        (ogg, duration)
                    }
                    Some("wav") => {
                        let b = match std::fs::read(&path) {
                            Ok(b) => b,
                            Err(e) => {
                                self.log_and_status(&format!("Failed to read WAV: {}", e));
                                return;
                            }
                        };
                        let dur = rodio::Decoder::new(std::io::Cursor::new(b.clone()))
                            .ok()
                            .and_then(|d| d.total_duration())
                            .unwrap_or(std::time::Duration::ZERO);
                        (b, dur)
                    }
                    _ => {
                        self.log_and_status("Unsupported audio format for playback");
                        return;
                    }
                }
            }
        };
        let cursor = std::io::Cursor::new(bytes.clone());
        let source = match rodio::Decoder::new(cursor) {
            Ok(s) => s,
            Err(e) => {
                self.log_and_status(&format!("Failed to decode audio: {}", e));
                return;
            }
        };
        if let Some(player) = &self.audio_player {
            player.stop();
        }
        let device = self.audio_sink.as_ref().unwrap();
        let player = rodio::Player::connect_new(device.mixer());
        player.append(source);
        if start_at > std::time::Duration::ZERO {
            let _ = player.try_seek(start_at);
        }
        player.play();
        self.audio_player = Some(player);
        self.player_source = Some(path);
        self.player_bytes = Some(bytes);
        self.player_duration = source_duration;
        self.player_modal_open = true;
        self.player_play_started = Some(Instant::now());
        self.player_paused_at = start_at;
    }

    // reports playhead position based off track duration
    pub(super) fn current_position(&self) -> std::time::Duration {
        let base = self.player_paused_at;
        match self.player_play_started {
            Some(t) => base + t.elapsed(),
            None => base,
        }
        .min(self.player_duration)
    }

    // pause/resume and starts from beginning if track has finished
    pub(super) fn toggle_playback(&mut self) {
        let finished = self.audio_player.as_ref().map(|p| p.empty()).unwrap_or(true);
        if finished {
            if let Some(path) = self.player_source.clone() {
                self.start_player(path);
            }
            return;
        }
        let Some(player) = &self.audio_player else { return };
        if player.is_paused() {
            player.play();
            self.player_play_started = Some(Instant::now());
        } else {
            player.pause();
            if let Some(started) = self.player_play_started.take() {
                self.player_paused_at += started.elapsed();
            }
        }
    }

    // seeks by delta, backwards seeking rebuilds player because rodio ogg seek isn't reliable enough
    pub(super) fn seek_relative(&mut self, delta_secs: f32) {
        let target = if delta_secs < 0.0 {
            self.current_position()
                .saturating_sub(std::time::Duration::from_secs_f32(-delta_secs))
        } else {
            (self.current_position() + std::time::Duration::from_secs_f32(delta_secs))
                .min(self.player_duration)
        };
        if delta_secs < 0.0 {
            let was_paused = self
                .audio_player
                .as_ref()
                .map(|p| p.is_paused())
                .unwrap_or(true);
            let path = self.player_source.clone();
            if let Some(path) = path {
                self.start_player_at(path, target);
                if was_paused {
                    if let Some(p) = &self.audio_player {
                        p.pause();
                    }
                    self.player_play_started = None;
                }
            }
        } else {
            let Some(player) = &self.audio_player else { return };
            match player.try_seek(target) {
                Ok(_) => {
                    self.player_paused_at = target;
                    if !player.is_paused() {
                        self.player_play_started = Some(Instant::now());
                    }
                }
                Err(e) => self.log_and_status(&format!("Seek failed: {}", e)),
            }
        }
    }
}