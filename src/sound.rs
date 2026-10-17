//! Retro sound effects, synthesized at startup (no sound files needed).

use macroquad::audio::{load_sound_from_bytes, play_sound, PlaySoundParams, Sound};

const RATE: u32 = 22050;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    Step,
    Turn,
    Mine,
    Place,
    Collect,
    Bonk,
    Fail,
    Win,
    Click,
    Add,
}

const ALL: [Sfx; 10] =
    [Sfx::Step, Sfx::Turn, Sfx::Mine, Sfx::Place, Sfx::Collect, Sfx::Bonk, Sfx::Fail, Sfx::Win, Sfx::Click, Sfx::Add];

#[derive(Clone, Copy)]
enum Wave {
    Square,
    Triangle,
    Noise,
}

/// Each note is (frequency in Hz, length in seconds).
fn synth(notes: &[(f32, f32)], wave: Wave, vol: f32) -> Vec<f32> {
    let mut out = Vec::new();
    let mut seed: u32 = 12345;
    for &(freq, dur) in notes {
        let n = (dur * RATE as f32) as usize;
        for i in 0..n {
            let t = i as f32 / RATE as f32;
            let phase = (t * freq).fract();
            let v = match wave {
                Wave::Square => if phase < 0.5 { 1.0 } else { -1.0 },
                Wave::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
                Wave::Noise => {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
                }
            };
            // Quick fade in/out so notes don't click.
            let env = (i as f32 / 80.0).min(1.0) * ((n - i) as f32 / (n as f32 * 0.6)).min(1.0);
            out.push(v * env * vol);
        }
    }
    out
}

fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&RATE.to_le_bytes());
    b.extend_from_slice(&(RATE * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes());
    }
    b
}

fn samples(s: Sfx) -> Vec<f32> {
    use Wave::*;
    match s {
        Sfx::Step => synth(&[(180.0, 0.05)], Triangle, 0.35),
        Sfx::Turn => synth(&[(520.0, 0.035)], Triangle, 0.25),
        Sfx::Mine => synth(&[(0.0, 0.12)], Noise, 0.25),
        Sfx::Place => {
            let mut v = synth(&[(110.0, 0.06)], Square, 0.15);
            v.extend(synth(&[(0.0, 0.05)], Noise, 0.12));
            v
        }
        Sfx::Collect => synth(&[(660.0, 0.06), (880.0, 0.06), (1320.0, 0.12)], Square, 0.12),
        Sfx::Bonk => synth(&[(98.0, 0.22)], Square, 0.2),
        Sfx::Fail => synth(&[(392.0, 0.15), (330.0, 0.15), (262.0, 0.32)], Triangle, 0.35),
        Sfx::Win => synth(
            &[(523.0, 0.1), (659.0, 0.1), (784.0, 0.1), (1047.0, 0.18), (784.0, 0.08), (1047.0, 0.35)],
            Square,
            0.12,
        ),
        Sfx::Click => synth(&[(900.0, 0.025)], Triangle, 0.2),
        Sfx::Add => synth(&[(600.0, 0.03), (900.0, 0.04)], Triangle, 0.25),
    }
}

pub struct Sounds {
    sounds: Vec<Option<Sound>>,
    pub muted: bool,
}

impl Sounds {
    pub async fn load() -> Sounds {
        let mut sounds = Vec::new();
        for s in ALL {
            sounds.push(load_sound_from_bytes(&wav(&samples(s))).await.ok());
        }
        Sounds { sounds, muted: false }
    }

    pub fn play(&self, s: Sfx) {
        if self.muted {
            return;
        }
        if let Some(Some(snd)) = self.sounds.get(s as usize) {
            play_sound(snd, PlaySoundParams { looped: false, volume: 0.8 });
        }
    }
}
