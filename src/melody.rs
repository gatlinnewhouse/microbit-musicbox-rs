use defmt::Format;

use crate::tone::Tone;

#[derive(Format, Debug)]
pub struct Melody {
    whole_note_delay_ms: u32,
    notes: &'static [(Tone, i8)],
}

impl Melody {
    pub fn get(&self, pos: usize) -> Option<(Tone, u32)> {
        self.notes.get(pos).cloned().map(|(note, div)| {
            let dotted = if div > 0 { false } else { true };
            let div = div.abs() as f32;
            let delay_ms = self.whole_note_delay_ms as f32 / div;
            (note, if dotted { delay_ms * 1.5 } else { delay_ms } as u32)
        })
    }
}

macro_rules! melody {
    (
        name = $name:ident,
        tempo = $tempo:expr,
        beat = $beat:expr,
        $([$($note:ident: $duration:expr),*]),*
    ) => {
        pub const $name: Melody = Melody {
            whole_note_delay_ms: (60000 * $beat) / $tempo,
            notes: &[
                $(
                    $((Tone::$note, $duration),)*
                )*
            ]
        };
    };
}

// Happy birthday
// https://musescore.com/user/8221/scores/26906
melody!(
    name = HAPPY_BIRTHDAY, tempo = 140, beat = 4,
    [C4:4, C4:8, D4:-4, C4:-4, F4:-4, E4:-2],
    [C4:4, C4:8, D4:-4, C4:-4, G4:-4, F4:-2],
    [C4:4, C4:8, C5:-4, A4:-4, F4:-4, E4:-4, D4:-4],
    [AS4:4, AS4:8, A4:-4, F4:-4, G4:-4, F4:-2]
);
