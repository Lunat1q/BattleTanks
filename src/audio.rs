//! Sound effects synthesised at startup into in-memory WAVs (no asset files). `Sfx` messages play them.

use crate::game::Sfx;
use crate::rng::Rng;
use crate::settings::Settings;
use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::f32::consts::TAU;

const RATE: u32 = 22_050;

#[derive(Resource)]
struct Sounds(HashMap<Sfx, Handle<AudioSource>>);

pub fn plugin(app: &mut App) {
    // Silent for smoke runs (`BT_SHOT`) and on request (`BT_MUTE=1`), so testing doesn't blast the room.
    if std::env::var_os("BT_SHOT").is_some() || std::env::var_os("BT_MUTE").is_some() {
        return;
    }
    app.add_systems(Startup, build).add_systems(PostUpdate, play);
}

/// 16-bit mono PCM WAV.
pub fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32_000.0) as i16).to_le_bytes());
    }
    out
}

/// One voice: frequency sweep `f0 -> f1` with an exponential-ish decay.
struct Voice {
    f0: f32,
    f1: f32,
    len: f32,
    square: f32, // 0 = sine, 1 = square
    noise: f32,
    vol: f32,
}

fn synth(voices: &[(f32, Voice)], rng: &mut Rng) -> Vec<f32> {
    let total = voices.iter().map(|(at, v)| at + v.len).fold(0.0, f32::max);
    let mut out = vec![0.0f32; (total * RATE as f32) as usize + 1];
    for (at, v) in voices {
        let start = (at * RATE as f32) as usize;
        let n = (v.len * RATE as f32) as usize;
        let mut phase = 0.0f32;
        let mut lp = 0.0f32;
        for i in 0..n {
            let t = i as f32 / n as f32;
            phase += (v.f0 + (v.f1 - v.f0) * t) / RATE as f32;
            let sine = (phase * TAU).sin();
            let tone = sine * (1.0 - v.square) + sine.signum() * v.square * 0.6;
            lp += (rng.range(-1.0, 1.0) - lp) * 0.35;
            let env = (1.0 - t).powf(1.6) * (i as f32 / 60.0).min(1.0);
            out[start + i] += (tone * (1.0 - v.noise) + lp * v.noise * 1.6) * env * v.vol;
        }
    }
    out
}

fn v(f0: f32, f1: f32, len: f32, square: f32, noise: f32, vol: f32) -> Voice {
    Voice { f0, f1, len, square, noise, vol }
}

/// Short melody of `(semitones above A4, length)` notes.
fn melody(notes: &[(i32, f32)], square: f32, vol: f32) -> Vec<(f32, Voice)> {
    let mut t = 0.0;
    notes
        .iter()
        .map(|&(n, len)| {
            let f = 440.0 * 2f32.powf(n as f32 / 12.0);
            let at = t;
            t += len;
            (at, v(f, f, len * 1.1, square, 0.0, vol))
        })
        .collect()
}

fn recipe(s: Sfx) -> Vec<(f32, Voice)> {
    match s {
        Sfx::Shoot => vec![(0.0, v(900.0, 180.0, 0.09, 0.8, 0.3, 0.35))],
        Sfx::Brick => vec![(0.0, v(300.0, 80.0, 0.12, 0.2, 0.85, 0.5))],
        Sfx::Steel => vec![(0.0, v(1800.0, 1500.0, 0.1, 1.0, 0.2, 0.25)), (0.0, v(2700.0, 2600.0, 0.06, 0.0, 0.0, 0.2))],
        Sfx::ArmorHit => vec![(0.0, v(700.0, 500.0, 0.08, 1.0, 0.3, 0.3))],
        Sfx::EnemyBoom => vec![(0.0, v(160.0, 40.0, 0.45, 0.3, 0.8, 0.7))],
        Sfx::PlayerBoom => vec![(0.0, v(220.0, 30.0, 0.8, 0.5, 0.75, 0.8))],
        Sfx::BaseBoom => vec![(0.0, v(120.0, 25.0, 1.4, 0.4, 0.8, 0.9)), (0.1, v(80.0, 30.0, 1.2, 0.0, 0.5, 0.6))],
        Sfx::PowerAppear => melody(&[(3, 0.06), (7, 0.06), (10, 0.06), (15, 0.12)], 0.6, 0.25),
        Sfx::PowerPick => melody(&[(12, 0.05), (16, 0.05), (19, 0.05), (24, 0.15)], 0.5, 0.3),
        Sfx::ExtraLife => melody(&[(7, 0.08), (12, 0.08), (7, 0.08), (12, 0.08), (19, 0.25)], 0.4, 0.3),
        Sfx::Pause => melody(&[(12, 0.07), (7, 0.07), (12, 0.12)], 0.8, 0.25),
        Sfx::StageStart => melody(&[(-2, 0.12), (3, 0.12), (7, 0.12), (10, 0.12), (7, 0.12), (10, 0.12), (15, 0.35)], 0.5, 0.25),
        Sfx::GameOver => melody(&[(7, 0.2), (5, 0.2), (3, 0.2), (2, 0.2), (-2, 0.6)], 0.5, 0.3),
        Sfx::MenuMove => vec![(0.0, v(660.0, 660.0, 0.04, 1.0, 0.0, 0.15))],
        Sfx::MenuPick => melody(&[(7, 0.05), (19, 0.1)], 0.8, 0.22),
        // Heavy grind then a rising chord: a bastion gate rolls open.
        Sfx::GateOpen => {
            let mut g = vec![(0.0, v(90.0, 60.0, 0.5, 0.6, 0.6, 0.5))];
            g.extend(melody(&[(0, 0.1), (7, 0.1), (12, 0.25)], 0.5, 0.25).into_iter().map(|(t, n)| (t + 0.35, n)));
            g
        }
        Sfx::BossBoom => vec![(0.0, v(140.0, 20.0, 1.8, 0.5, 0.85, 1.0)), (0.2, v(90.0, 25.0, 1.6, 0.2, 0.6, 0.7))],
    }
}

const ALL: [Sfx; 17] = [
    Sfx::Shoot,
    Sfx::Brick,
    Sfx::Steel,
    Sfx::ArmorHit,
    Sfx::EnemyBoom,
    Sfx::PlayerBoom,
    Sfx::BaseBoom,
    Sfx::PowerAppear,
    Sfx::PowerPick,
    Sfx::ExtraLife,
    Sfx::Pause,
    Sfx::StageStart,
    Sfx::GameOver,
    Sfx::MenuMove,
    Sfx::MenuPick,
    Sfx::GateOpen,
    Sfx::BossBoom,
];

fn build(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    let mut rng = Rng::new(42);
    let map = ALL.iter().map(|&s| (s, sources.add(AudioSource { bytes: wav(&synth(&recipe(s), &mut rng)).into() }))).collect();
    commands.insert_resource(Sounds(map));
}

fn play(mut commands: Commands, sounds: Option<Res<Sounds>>, settings: Res<Settings>, mut events: MessageReader<Sfx>) {
    let Some(sounds) = sounds else { return };
    let volume = settings.volume as f32 / 100.0;
    if volume <= 0.0 {
        events.clear();
        return;
    }
    let mut played: Vec<Sfx> = Vec::new();
    for &s in events.read() {
        if played.contains(&s) {
            continue; // one of each per frame, so a grenade isn't deafening
        }
        played.push(s);
        if let Some(h) = sounds.0.get(&s) {
            commands.spawn((AudioPlayer(h.clone()), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume))));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_matches_data() {
        let bytes = wav(&[0.0, 0.5, -0.5]);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(bytes.len(), 44 + 6);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 6);
    }

    #[test]
    fn every_sound_synthesises() {
        let mut rng = Rng::new(1);
        for s in ALL {
            let samples = synth(&recipe(s), &mut rng);
            assert!(samples.len() > 100, "{s:?}");
            assert!(samples.iter().all(|x| x.is_finite()));
        }
    }
}
