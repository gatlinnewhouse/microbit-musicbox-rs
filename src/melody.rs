use defmt::Format;

use crate::note::Note;

#[derive(Format, Debug)]
pub struct Melody<'a> {
    whole_note_duration: u32,
    notes: &'a [(Note, i8)],
}

impl<'a> Melody<'a> {
    pub fn get_note(&self, pos: usize) -> Option<(Note, f32)> {
        self.notes.get(pos).cloned().map(|(note, div)| {
            let dotted = if div > 0 { false } else { true };
            let div = div.abs() as f32;
            let duration = self.whole_note_duration as f32 / div;
            (note, if dotted { duration * 1.5 } else { duration })
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
        pub const $name: Melody<'static> = Melody {
            whole_note_duration: (60000 * $beat) / $tempo,
            notes: &[
                $(
                    $((Note::$note, $duration),)*
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
