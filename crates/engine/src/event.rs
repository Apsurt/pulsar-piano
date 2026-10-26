//! MIDI is the internal event format everywhere: hardware, on-screen keys and computer keys
//! all produce the same `MidiEvent`.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MidiEvent {
    NoteOn {
        channel: u8,
        key: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        key: u8,
        velocity: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    ChannelPressure {
        channel: u8,
        value: u8,
    },
    /// 14-bit value, 8192 is centre.
    PitchBend {
        channel: u8,
        value: u16,
    },
}

pub const CC_SUSTAIN: u8 = 64;

impl MidiEvent {
    /// Parses one complete channel message. System messages (clock, sysex, active sensing) are ignored.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let (&status, data) = bytes.split_first()?;
        let channel = status & 0x0F;
        let d = |i: usize| data.get(i).copied().filter(|b| *b < 0x80);
        Some(match status & 0xF0 {
            0x80 => MidiEvent::NoteOff {
                channel,
                key: d(0)?,
                velocity: d(1)?,
            },
            0x90 => match d(1)? {
                0 => MidiEvent::NoteOff {
                    channel,
                    key: d(0)?,
                    velocity: 0,
                },
                velocity => MidiEvent::NoteOn {
                    channel,
                    key: d(0)?,
                    velocity,
                },
            },
            0xB0 => MidiEvent::ControlChange {
                channel,
                controller: d(0)?,
                value: d(1)?,
            },
            0xC0 => MidiEvent::ProgramChange {
                channel,
                program: d(0)?,
            },
            0xD0 => MidiEvent::ChannelPressure {
                channel,
                value: d(0)?,
            },
            0xE0 => MidiEvent::PitchBend {
                channel,
                value: d(0)? as u16 | (d(1)? as u16) << 7,
            },
            _ => return None,
        })
    }

    /// Status byte and two data bytes (unused data bytes are 0).
    pub fn to_bytes(self) -> [u8; 3] {
        match self {
            MidiEvent::NoteOff {
                channel,
                key,
                velocity,
            } => [0x80 | channel, key, velocity],
            MidiEvent::NoteOn {
                channel,
                key,
                velocity,
            } => [0x90 | channel, key, velocity],
            MidiEvent::ControlChange {
                channel,
                controller,
                value,
            } => [0xB0 | channel, controller, value],
            MidiEvent::ProgramChange { channel, program } => [0xC0 | channel, program, 0],
            MidiEvent::ChannelPressure { channel, value } => [0xD0 | channel, value, 0],
            MidiEvent::PitchBend { channel, value } => {
                [0xE0 | channel, (value & 0x7F) as u8, (value >> 7) as u8]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_on_with_zero_velocity_is_note_off() {
        assert_eq!(
            MidiEvent::parse(&[0x91, 60, 0]),
            Some(MidiEvent::NoteOff {
                channel: 1,
                key: 60,
                velocity: 0
            })
        );
    }

    #[test]
    fn round_trips() {
        for bytes in [
            [0x90, 60, 100],
            [0x80, 61, 40],
            [0xB0, 64, 127],
            [0xE3, 0x12, 0x40],
        ] {
            assert_eq!(MidiEvent::parse(&bytes).unwrap().to_bytes(), bytes);
        }
    }

    #[test]
    fn rejects_system_and_truncated_messages() {
        assert_eq!(MidiEvent::parse(&[0xF8]), None);
        assert_eq!(MidiEvent::parse(&[0xFE]), None);
        assert_eq!(MidiEvent::parse(&[0x90, 60]), None);
        assert_eq!(MidiEvent::parse(&[]), None);
    }
}
