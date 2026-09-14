use std::collections::HashMap;

use tenui_core::color::Color;

/// Hardware action dispatched by physical controllers (faders, knobs, keypads).
#[derive(Debug, Clone, PartialEq)]
pub enum HardwareAction {
    SliderMoved { index: u8, value: f32 },
    RotaryTurned { index: u8, delta_velocity: f32 },
    ButtonPressed { key_id: u8 },
    ButtonReleased { key_id: u8 },
}

/// Normalizes 7-bit MIDI Control Change (0..127) to scalar range [0.0, 1.0].
#[inline]
pub fn normalize_midi_7bit(raw: u8) -> f32 {
    (raw.min(127) as f32) / 127.0
}

/// Normalizes 14-bit high-resolution MIDI (MSB + LSB) to scalar range [0.0, 1.0].
#[inline]
pub fn normalize_midi_14bit(msb: u8, lsb: u8) -> f32 {
    let combined = (((msb as u16) & 0x7F) << 7) | ((lsb as u16) & 0x7F);
    (combined.min(16383) as f32) / 16383.0
}

/// Normalizes 16-bit USB HID analog input (0..65535) to scalar range [0.0, 1.0].
#[inline]
pub fn normalize_hid_16bit(raw: u16) -> f32 {
    (raw as f32) / 65535.0
}

/// Continuous kinetic impulse controller translating physical detented rotary dials into smooth spring momentum.
#[derive(Debug, Clone)]
pub struct RotaryImpulseController {
    pub sensitivity: f32,
    pub current_velocity: f32,
    pub friction: f32,
}

impl RotaryImpulseController {
    pub fn new(sensitivity: f32) -> Self {
        Self {
            sensitivity,
            current_velocity: 0.0,
            friction: 5.0, // default damping coefficient
        }
    }

    /// Translates rotary dial ticks & velocity into a continuous kinetic impulse:
    /// Δv = k * v_dial * ticks
    pub fn translate_rotary_to_kinetic_impulse(&mut self, ticks: i32, dial_velocity: f32) -> f32 {
        let impulse = (ticks as f32) * dial_velocity * self.sensitivity;
        self.current_velocity += impulse;
        impulse
    }

    /// Advances simulation by `dt_secs` under exponential friction damping.
    /// Returns the position displacement (delta displacement) during this frame.
    pub fn step(&mut self, dt_secs: f32) -> f32 {
        let displacement = self.current_velocity * dt_secs;
        // Apply friction decay
        let decay = (-self.friction * dt_secs).exp();
        self.current_velocity *= decay;
        if self.current_velocity.abs() < 1e-4 {
            self.current_velocity = 0.0;
        }
        displacement
    }

    /// Stops all kinetic motion immediately.
    pub fn stop(&mut self) {
        self.current_velocity = 0.0;
    }
}

/// Bidirectional hardware key LED feedback manager (e.g. StreamDeck, Loupedeck RGB keycaps).
#[derive(Debug, Default, Clone)]
pub struct KeyLedFeedback {
    led_colors: HashMap<u8, Color>,
}

impl KeyLedFeedback {
    pub fn new() -> Self {
        Self {
            led_colors: HashMap::new(),
        }
    }

    /// Sets the hardware key LED color.
    pub fn set_key_led(&mut self, key_id: u8, color: Color) {
        self.led_colors.insert(key_id, color);
    }

    /// Gets current color configured for the given key.
    pub fn get_key_led(&self, key_id: u8) -> Option<Color> {
        self.led_colors.get(&key_id).copied()
    }

    /// Serializes key LED payload for hardware bus transmission:
    /// Format: `[0xAA (Header), key_id, R, G, B, 0xFF (Footer)]`
    pub fn serialize_packet(&self, key_id: u8) -> Option<Vec<u8>> {
        let color = self.led_colors.get(&key_id)?;
        let (r, g, b) = match color {
            Color::Rgb(r, g, b) => (*r, *g, *b),
            Color::Black => (0, 0, 0),
            Color::White => (255, 255, 255),
            Color::Red => (255, 0, 0),
            Color::Green => (0, 255, 0),
            Color::Blue => (0, 0, 255),
            Color::Yellow => (255, 255, 0),
            Color::Cyan => (0, 255, 255),
            Color::Magenta => (255, 0, 255),
            _ => (128, 128, 128),
        };
        Some(vec![0xAA, key_id, r, g, b, 0xFF])
    }
}

/// Central hardware event reactor decoupling raw HID/MIDI polling from the TUI render loop.
#[derive(Debug, Default)]
pub struct HidEventReactor {
    queue: Vec<HardwareAction>,
    rotaries: HashMap<u8, RotaryImpulseController>,
    pub led_feedback: KeyLedFeedback,
}

impl HidEventReactor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueues a hardware action.
    pub fn push_action(&mut self, action: HardwareAction) {
        self.queue.push(action);
    }

    /// Drains all pending hardware actions for dispatching into UI signal graphs.
    pub fn drain_actions(&mut self) -> Vec<HardwareAction> {
        std::mem::take(&mut self.queue)
    }

    /// Ingests a raw 7-bit MIDI Continuous Controller (CC) packet.
    pub fn process_midi_cc(&mut self, controller: u8, raw_val: u8) {
        let value = normalize_midi_7bit(raw_val);
        self.push_action(HardwareAction::SliderMoved {
            index: controller,
            value,
        });
    }

    /// Ingests a raw 16-bit USB HID slider / fader report.
    pub fn process_hid_fader(&mut self, fader_id: u8, raw_16bit: u16) {
        let value = normalize_hid_16bit(raw_16bit);
        self.push_action(HardwareAction::SliderMoved { index: fader_id, value });
    }

    /// Ingests a rotary encoder event and computes kinetic impulse.
    pub fn process_rotary_tick(&mut self, knob_id: u8, ticks: i32, dial_velocity: f32, sensitivity: f32) {
        let rotary = self
            .rotaries
            .entry(knob_id)
            .or_insert_with(|| RotaryImpulseController::new(sensitivity));
        let delta_v = rotary.translate_rotary_to_kinetic_impulse(ticks, dial_velocity);
        self.push_action(HardwareAction::RotaryTurned {
            index: knob_id,
            delta_velocity: delta_v,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalization() {
        assert_eq!(normalize_midi_7bit(0), 0.0);
        assert!((normalize_midi_7bit(127) - 1.0).abs() < 1e-6);
        assert!((normalize_midi_7bit(64) - 0.5039).abs() < 0.01);

        assert_eq!(normalize_hid_16bit(0), 0.0);
        assert_eq!(normalize_hid_16bit(65535), 1.0);
    }

    #[test]
    fn test_rotary_kinetic_momentum() {
        let mut controller = RotaryImpulseController::new(2.0);
        let impulse = controller.translate_rotary_to_kinetic_impulse(3, 1.5);
        // Δv = 3 * 1.5 * 2.0 = 9.0
        assert!((impulse - 9.0).abs() < 1e-5);
        assert_eq!(controller.current_velocity, 9.0);

        // Step over time
        let disp = controller.step(0.016);
        assert!(disp > 0.0);
        assert!(controller.current_velocity < 9.0); // damped
    }

    #[test]
    fn test_key_led_feedback() {
        let mut led = KeyLedFeedback::new();
        led.set_key_led(5, Color::Rgb(10, 20, 30));
        let packet = led.serialize_packet(5).expect("packet exists");
        assert_eq!(packet, vec![0xAA, 5, 10, 20, 30, 0xFF]);
    }
}
