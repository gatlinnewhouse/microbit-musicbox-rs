use core::{fmt::Display, str::FromStr};

use self::error::Error;

#[derive(Debug)]
pub enum Key {
    C,
    Cs,
    Df,
    D,
    Ds,
    Ef,
    E,
    F,
    Fs,
    Gf,
    G,
    Gs,
    Af,
    A,
    Bf,
    B,
}

impl Display for Key {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use Key::*;
        let symbol = match *self {
            C => "C",
            Cs => "C♯",
            Df => "D♭",
            D => "D",
            Ds => "D♯",
            E => "E",
            Ef => "E♭",
            F => "F",
            Fs => "F♯",
            Gf => "G♭",
            G => "G",
            Gs => "G♯",
            Af => "A♭",
            A => "A",
            Bf => "B♭",
            B => "B",
        };

        write!(f, "{}", symbol)
    }
}

impl FromStr for Key {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Key::*;

        if s.is_empty() {
            return Err(Error::EmptyKey);
        }

        let key = match s {
            "C" => C,
            "C♯" => Cs,
            "D♭" => Df,
            "D" => D,
            "D♯" => Ds,
            "E" => E,
            "E♭" => Ef,
            "F" => F,
            "F♯" => Fs,
            "G♭" => Gf,
            "G" => G,
            "G♯" => Gs,
            "A♭" => Af,
            "A" => A,
            "B♭" => Bf,
            "B" => B,
            ks => return Err(Error::InvalidKey),
        };
        Ok(key)
    }
}

#[derive(Debug)]
pub enum Duration {
    /// 四分音符
    Quarter,
    /// 八分音符
    Eighth,
    /// 十六分音符
    Sixteenth,
}

impl Display for Duration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            Duration::Quarter => "4",
            Duration::Eighth => "8",
            Duration::Sixteenth => "16",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for Duration {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Duration::*;

        if s.is_empty() {
            return Err(Error::EmptyDuration);
        }

        let duration = match s {
            "4" => Quarter,
            "8" => Eighth,
            "16" => Sixteenth,
            s => return Err(Error::InvalidDuration),
        };
        Ok(duration)
    }
}

#[derive(Debug)]
pub struct Note {
    /// 音名
    key: Key,
    /// 八度, 取值 0-9, 如: C4 = C major
    octave: u8,
    /// 附点音符
    dotted: bool,
    /// 音符持续时间
    duration: Duration,
}

impl Display for Note {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let dotted = if self.dotted { "-" } else { "" };
        write!(f, "{}{}:{}{}", self.key, self.octave, dotted, self.duration)
    }
}

impl FromStr for Note {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(Error::EmptyNote);
        }

        if let Some(colon) = s.find(':') {
            let prefix = &s[0..colon];
            let prefix_len = prefix.len();
            let suffix = &s[colon + 1..];

            let key = prefix[..prefix_len].parse()?;
            let octave = prefix[prefix_len - 1..]
                .parse()
                .map_err(|_| Error::InvalidOctave)?;
            let dotted = &suffix[0..1] == "-";
            let duration = if dotted {
                suffix[1..].parse()?
            } else {
                suffix[..].parse()?
            };

            Ok(Note {
                key,
                octave,
                dotted,
                duration,
            })
        } else {
            Err(Error::MissingColon)
        }
    }
}

pub mod error {
    use core::fmt;
    use core::fmt::Display;

    pub enum Error {
        EmptyKey,
        InvalidKey,
        EmptyDuration,
        InvalidDuration,
        InvalidOctave,
        EmptyNote,
        MissingColon,
    }

    impl Error {
        #[doc(hidden)]
        pub fn __description(&self) -> &str {
            match &self {
                Error::EmptyKey => "cannot parse key from empty string",
                Error::InvalidKey => "invalid key found in string",
                Error::EmptyDuration => "cannot parse duration from empty string",
                Error::InvalidDuration => "invalid duration found in string",
                Error::InvalidOctave => "invalid octave found in string",
                Error::EmptyNote => "cannot parse note from empty string",
                Error::MissingColon => "missing ':' symbol found in string",
            }
        }
    }

    impl Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.__description().fmt(f)
        }
    }
}
