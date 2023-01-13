mod paser;

use defmt::Format;
use heapless::Vec;

pub struct Music<'a> {
    tempo: u32,
    melody: &'a str,
}

impl<'a> Music<'a> {
    pub const fn new(tempo: u32, melody: &'a str) -> Self {
        Self { tempo, melody }
    }
}

/// Happy Birthday
/// Score available at https://musescore.com/user/8221/scores/26906
const HAPPY_BIRTHDAY: Music = Music::new(
    140,
    "
    C4:4, C4:8,
    D4:-4, C4:-4, F4:-4,
    E4:-2, C4:4, C4:8,
    D4:-4, C4:-4, G4:-4,
    F4:-2, C4:4, C4:8,

    C5:-4, A4:-4, F4:-4,
    E4:-4, D4:-4, A#4:4, A#4:8,
    A4:-4, F4:-4, G4:-4,
    F4:-2",
);
