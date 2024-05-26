use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub struct MidiNote(pub u8);

impl MidiNote {
    pub fn from_midi_number(midi_number: u8) -> Self {
        Self(midi_number)
    }

    pub fn to_midi_number(self) -> u8 {
        self.0
    }

    pub fn from_frequency(frequency: f32) -> Self {
        let midi_number = 69.0 + 12.0 * (frequency / 440.0).log2();
        Self(midi_number as u8)
    }

    pub fn to_frequency(self) -> f32 {
        440.0 * 2.0_f32.powf((self.0 as f32 - 69.0) / 12.0)
    }
}

impl FromStr for MidiNote {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() < 2 {
            return Err(());
        }

        let note = match chars[0] {
            'C' => 0,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' => 11,
            _ => return Err(()),
        };

        if s.contains('#') {
            let note = (note + 1) % 12;
            let octave = s[2..].parse::<i32>().map_err(|_| ())? + 1;
            Ok(Self((octave * 12 + note) as u8))
        } else if s.contains('b') {
            let note = (note - 1) % 12;
            let octave = s[2..].parse::<i32>().map_err(|_| ())? + 1;
            Ok(Self((octave * 12 + note) as u8))
        } else {
            let octave = s[1..].parse::<i32>().map_err(|_| ())? + 1;
            Ok(Self((octave * 12 + note) as u8))
        }
    }
}
impl std::fmt::Display for MidiNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let note = self.0 % 12;
        let octave = self.0 / 12 - 1;
        let note_str = match note {
            0 => "C",
            1 => "C#",
            2 => "D",
            3 => "D#",
            4 => "E",
            5 => "F",
            6 => "F#",
            7 => "G",
            8 => "G#",
            9 => "A",
            10 => "A#",
            11 => "B",
            _ => unreachable!(),
        };
        write!(f, "{}{}", note_str, octave)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_note() {
        let midi_note = MidiNote::from_midi_number(69);
        assert_eq!(midi_note.to_frequency(), 440.0);

        let midi_note = MidiNote::from_frequency(440.0);
        assert_eq!(midi_note.to_midi_number(), 69);

        let midi_note = MidiNote::from_str("A4").unwrap();
        assert_eq!(midi_note.to_midi_number(), 69);

        let midi_note = MidiNote::from_str("A#4").unwrap();
        assert_eq!(midi_note.to_midi_number(), 70);

        let midi_note = MidiNote::from_str("Bb4").unwrap();
        assert_eq!(midi_note.to_midi_number(), 70);

        let midi_note = MidiNote::from_str("C5").unwrap();
        assert_eq!(midi_note.to_midi_number(), 72);
    }
}
