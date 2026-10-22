//! Builds the game's sound effects as WAV files in memory.
//!
//! Chrome's sounds are simple square-wave beeps, so they are synthesised here
//! rather than shipped as audio files.

const SAMPLE_RATE: u32 = 22_050;
/// Square waves are harsh at full scale, so they play well below it.
const VOLUME: f32 = 0.2;
/// Each note ramps in and out over this long, which avoids audible clicks.
const RAMP_SECONDS: f32 = 0.004;

/// A note: frequency in Hz and length in seconds.
type Note = (f32, f32);

pub const JUMP: &[Note] = &[(660.0, 0.06)];
pub const MILESTONE: &[Note] = &[(1046.0, 0.07), (1318.0, 0.12)];
pub const CRASH: &[Note] = &[(130.0, 0.08), (0.0, 0.03), (98.0, 0.16)];

/// Renders notes into a 16-bit mono WAV. A frequency of 0 is a rest.
pub fn wav(notes: &[Note]) -> Vec<u8> {
    let sample_count: usize = notes
        .iter()
        .map(|&(_, seconds)| (seconds * SAMPLE_RATE as f32) as usize)
        .sum();
    let data_len = (sample_count * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes per second
    out.extend_from_slice(&2u16.to_le_bytes()); // bytes per frame
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    let ramp = ((RAMP_SECONDS * SAMPLE_RATE as f32) as usize).max(1);
    for &(freq, seconds) in notes {
        let n = (seconds * SAMPLE_RATE as f32) as usize;
        for i in 0..n {
            let sample = if freq == 0.0 {
                0
            } else {
                let phase = (i as f32 * freq / SAMPLE_RATE as f32).fract();
                let square = if phase < 0.5 { 1.0 } else { -1.0 };
                let envelope = (i.min(n - 1 - i) as f32 / ramp as f32).min(1.0);
                (square * envelope * VOLUME * i16::MAX as f32) as i16
            };
            out.extend_from_slice(&sample.to_le_bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u32_at(b: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
    }

    fn samples(b: &[u8]) -> Vec<i16> {
        b[44..]
            .chunks(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect()
    }

    #[test]
    fn header_describes_the_data() {
        let w = wav(JUMP);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        assert_eq!(u32_at(&w, 4) as usize, w.len() - 8);
        assert_eq!(u32_at(&w, 24), SAMPLE_RATE);
        assert_eq!(u32_at(&w, 40) as usize, w.len() - 44);
    }

    #[test]
    fn length_matches_the_notes() {
        let w = wav(MILESTONE);
        let expected =
            ((0.07 * SAMPLE_RATE as f32) as usize) + ((0.12 * SAMPLE_RATE as f32) as usize);
        assert_eq!(samples(&w).len(), expected);
    }

    #[test]
    fn starts_and_ends_silent_and_stays_quiet() {
        let s = samples(&wav(CRASH));
        assert_eq!(s[0], 0);
        assert_eq!(*s.last().unwrap(), 0);
        let limit = (VOLUME * i16::MAX as f32) as i16;
        assert!(s.iter().all(|x| x.abs() <= limit));
        assert!(s.iter().any(|x| x.abs() == limit));
    }

    #[test]
    fn rests_are_silent() {
        let s = samples(&wav(&[(0.0, 0.01)]));
        assert!(s.iter().all(|&x| x == 0));
    }
}
