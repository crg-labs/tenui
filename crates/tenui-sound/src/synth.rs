use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundCue {
    KeycapDepress,
    KeycapRelease,
    FocusMove,
    ValidationFailure,
    TaskSuccess,
}

pub struct TactileAudioEngine {
    pub has_pcm_device: bool,
    pub has_dec_audio: bool,
    last_bell_time: Option<Instant>,
    bell_throttle: Duration,
}

impl TactileAudioEngine {
    pub fn new() -> Self {
        Self {
            has_pcm_device: false,
            // Conservative default: the Tier-2 DECPS sequences share the `CSI ... t`
            // form that xterm/dtterm read as *window manipulation* (e.g. `CSI 10 ; ... t`
            // toggles fullscreen). Leave DEC audio off until a caller opts in via
            // `with_dec_audio(true)` after confirming a DECPS-capable emulator.
            has_dec_audio: false,
            last_bell_time: None,
            bell_throttle: Duration::from_millis(100),
        }
    }

    pub fn with_dec_audio(mut self, enabled: bool) -> Self {
        self.has_dec_audio = enabled;
        self
    }

    pub fn with_pcm(mut self, enabled: bool) -> Self {
        self.has_pcm_device = enabled;
        self
    }

    pub fn with_bell_throttle(mut self, throttle: Duration) -> Self {
        self.bell_throttle = throttle;
        self
    }

    pub fn play_cue<W: Write>(&mut self, cue: SoundCue, stdout: &mut W) -> io::Result<()> {
        match cue {
            SoundCue::KeycapDepress => {
                if self.has_dec_audio {
                    // 850 Hz click for 8 milliseconds at moderate volume
                    stdout.write_all(b"\x1b[10;50;850;8t")?;
                    stdout.flush()?;
                }
            }
            SoundCue::KeycapRelease => {
                if self.has_dec_audio {
                    // Higher pitch spring return: 1150 Hz for 5 milliseconds
                    stdout.write_all(b"\x1b[10;30;1150;5t")?;
                    stdout.flush()?;
                }
            }
            SoundCue::FocusMove => {
                if self.has_dec_audio {
                    stdout.write_all(b"\x1b[10;20;600;3t")?;
                    stdout.flush()?;
                }
            }
            SoundCue::ValidationFailure => {
                if self.has_dec_audio {
                    // Double low dissonance
                    stdout.write_all(b"\x1b[10;80;220;50t")?;
                    stdout.flush()?;
                } else {
                    self.emit_throttled_bell(stdout)?;
                }
            }
            SoundCue::TaskSuccess => {
                if self.has_dec_audio {
                    // Ascending chime
                    stdout.write_all(b"\x1b[10;60;587;30t\x1b[10;60;880;40t")?;
                    stdout.flush()?;
                } else {
                    self.emit_throttled_bell(stdout)?;
                }
            }
        }
        Ok(())
    }

    fn emit_throttled_bell<W: Write>(&mut self, stdout: &mut W) -> io::Result<()> {
        let now = Instant::now();
        if let Some(last) = self.last_bell_time
            && now.duration_since(last) < self.bell_throttle
        {
            return Ok(());
        }
        stdout.write_all(b"\x07")?;
        stdout.flush()?;
        self.last_bell_time = Some(now);
        Ok(())
    }

    /// Analytical ADSR envelope calculation for a mechanical switch audio pulse:
    /// Returns amplitude scalar [0.0, 1.0].
    pub fn calculate_adsr(
        t: f32,
        attack_time: f32,
        decay_time: f32,
        sustain_level: f32,
        release_time: f32,
        total_duration: f32,
    ) -> f32 {
        if t < 0.0 || t > total_duration {
            0.0
        } else if t < attack_time {
            if attack_time > 0.0 {
                (t / attack_time).clamp(0.0, 1.0)
            } else {
                1.0
            }
        } else if t < attack_time + decay_time {
            if decay_time > 0.0 {
                let decay_progress = (t - attack_time) / decay_time;
                (1.0 - (1.0 - sustain_level) * decay_progress).clamp(0.0, 1.0)
            } else {
                sustain_level
            }
        } else if t < total_duration - release_time {
            sustain_level.clamp(0.0, 1.0)
        } else if release_time > 0.0 {
            let release_progress = (t - (total_duration - release_time)) / release_time;
            (sustain_level * (1.0 - release_progress)).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

impl Default for TactileAudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decps_audio_escapes() {
        let mut engine = TactileAudioEngine::new().with_dec_audio(true);
        let mut output = Vec::new();

        engine.play_cue(SoundCue::KeycapDepress, &mut output).unwrap();
        assert_eq!(output, b"\x1b[10;50;850;8t");

        output.clear();
        engine.play_cue(SoundCue::KeycapRelease, &mut output).unwrap();
        assert_eq!(output, b"\x1b[10;30;1150;5t");

        output.clear();
        engine.play_cue(SoundCue::FocusMove, &mut output).unwrap();
        assert_eq!(output, b"\x1b[10;20;600;3t");

        output.clear();
        engine.play_cue(SoundCue::ValidationFailure, &mut output).unwrap();
        assert_eq!(output, b"\x1b[10;80;220;50t");

        output.clear();
        engine.play_cue(SoundCue::TaskSuccess, &mut output).unwrap();
        assert_eq!(output, b"\x1b[10;60;587;30t\x1b[10;60;880;40t");
    }

    #[test]
    fn test_bell_fallback_and_throttle() {
        let mut engine = TactileAudioEngine::new()
            .with_dec_audio(false)
            .with_bell_throttle(Duration::from_millis(50));
        let mut output = Vec::new();

        // First failure triggers bell
        engine.play_cue(SoundCue::ValidationFailure, &mut output).unwrap();
        assert_eq!(output, b"\x07");

        // Immediate second failure within throttle period is muted
        output.clear();
        engine.play_cue(SoundCue::ValidationFailure, &mut output).unwrap();
        assert!(output.is_empty());
    }

    #[test]
    fn test_adsr_envelope() {
        // Total duration: 0.1s. Attack: 0.01s, Decay: 0.02s, Sustain: 0.7, Release: 0.02s
        let attack = 0.01;
        let decay = 0.02;
        let sustain = 0.7;
        let release = 0.02;
        let total = 0.1;

        let a = TactileAudioEngine::calculate_adsr(0.005, attack, decay, sustain, release, total);
        assert!((a - 0.5).abs() < 0.01);

        let peak = TactileAudioEngine::calculate_adsr(0.01, attack, decay, sustain, release, total);
        assert!((peak - 1.0).abs() < 0.01);

        let s = TactileAudioEngine::calculate_adsr(0.05, attack, decay, sustain, release, total);
        assert!((s - 0.7).abs() < 0.01);

        let after = TactileAudioEngine::calculate_adsr(0.15, attack, decay, sustain, release, total);
        assert_eq!(after, 0.0);
    }
}
