//! Touch sensitivity: how hard you have to play for a given loudness.
//! Applied to keyboard note-ons on their way to the instrument, so it adds no latency.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Touch {
    /// Soft presses come out louder: for light or small keys.
    Light,
    /// Velocity exactly as the keyboard sends it.
    #[default]
    Normal,
    /// You have to dig in to get loud.
    Heavy,
    /// Every note at the same velocity.
    Fixed,
}

impl Touch {
    /// Maps a note-on velocity (1..=127). 0 stays 0: it means note-off.
    pub fn apply(self, velocity: u8) -> u8 {
        let velocity = velocity.min(127);
        if velocity == 0 {
            return 0;
        }
        let exponent = match self {
            Touch::Normal => return velocity,
            Touch::Fixed => return 100,
            Touch::Light => 0.5,
            Touch::Heavy => 1.6,
        };
        let curved = (velocity as f32 / 127.0).powf(exponent);
        (1.0 + curved * 126.0).round() as u8
    }

    pub(crate) fn to_u8(self) -> u8 {
        self as u8
    }

    pub(crate) fn from_u8(value: u8) -> Self {
        match value {
            0 => Touch::Light,
            2 => Touch::Heavy,
            3 => Touch::Fixed,
            _ => Touch::Normal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_is_untouched_and_ends_are_kept() {
        for v in 0..=127 {
            assert_eq!(Touch::Normal.apply(v), v);
        }
        for touch in [Touch::Light, Touch::Heavy] {
            assert_eq!(touch.apply(0), 0, "note-off must stay note-off");
            assert_eq!(touch.apply(127), 127);
            assert!(touch.apply(1) >= 1);
        }
    }

    #[test]
    fn light_lifts_soft_notes_and_heavy_lowers_them() {
        assert!(Touch::Light.apply(20) > 50);
        assert!(Touch::Heavy.apply(64) < 50);
        assert_eq!(Touch::Fixed.apply(5), 100);
    }

    #[test]
    fn curves_never_reorder_velocities() {
        for touch in [Touch::Light, Touch::Heavy] {
            for v in 1..127 {
                assert!(touch.apply(v) <= touch.apply(v + 1));
            }
        }
    }

    #[test]
    fn round_trips_through_u8() {
        for touch in [Touch::Light, Touch::Normal, Touch::Heavy, Touch::Fixed] {
            assert_eq!(Touch::from_u8(touch.to_u8()), touch);
        }
    }
}
