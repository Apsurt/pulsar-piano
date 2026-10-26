use crate::MidiEvent;

/// A sound source. Both methods run on the audio thread: they must not block, lock or allocate.
pub trait Instrument: Send {
    fn handle(&mut self, event: MidiEvent);
    /// Overwrites `left` and `right` (same length) with the next block of audio.
    fn render(&mut self, left: &mut [f32], right: &mut [f32]);
}
