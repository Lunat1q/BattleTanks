//! Particle effects driven by `Fx` messages, screen shake, floating score numbers.

use super::{Gfx, MainCamera, z};
use crate::game::{Fx, FxKind, Paused, Screen};
use crate::rng::Rng;
use bevy::prelude::*;

#[derive(Component)]
struct Particle {
    vel: Vec2,
    drag: f32,
    age: f32,
    life: f32,
    size: (f32, f32),
    color: (LinearRgba, LinearRgba),
}

/// Camera shake "trauma", 0..1.
#[derive(Resource, Default)]
pub struct Shake(pub f32);

pub fn plugin(app: &mut App) {
    app.init_resource::<Shake>()
        .add_systems(Update, (spawn_fx, update_particles, shake_camera).chain().after(crate::game::Sim).run_if(|p: Res<Paused>| !p.0));
}

fn lerp(a: LinearRgba, b: LinearRgba, t: f32) -> LinearRgba {
    LinearRgba::new(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        a.alpha + (b.alpha - a.alpha) * t,
    )
}

fn lin(r: f32, g: f32, b: f32, a: f32) -> LinearRgba {
    LinearRgba::new(r, g, b, a)
}

pub fn spawn_sparkle(commands: &mut Commands, gfx: &Gfx, pos: Vec2) {
    commands.spawn((
        Sprite { image: gfx.sparkle.clone(), custom_size: Some(Vec2::splat(16.0)), ..default() },
        Transform::from_translation(pos.extend(z::FX)),
        Particle {
            vel: Vec2::ZERO,
            drag: 1.0,
            age: 0.0,
            life: crate::rules::SPAWN_TIME,
            size: (16.0, 16.0),
            color: (lin(1.6, 1.8, 2.4, 1.0), lin(1.6, 1.8, 2.4, 1.0)),
        },
        Sparkle,
        DespawnOnExit(Screen::Stage),
    ));
}

/// Pulses and spins instead of fading.
#[derive(Component)]
struct Sparkle;

struct Burst {
    count: u32,
    speed: (f32, f32),
    life: (f32, f32),
    size: (f32, f32),
    color: (LinearRgba, LinearRgba),
    drag: f32,
    /// Base direction and spread (radians); full circle when `None`.
    cone: Option<(Vec2, f32)>,
}

fn burst(commands: &mut Commands, gfx: &Gfx, rng: &mut Rng, pos: Vec2, b: Burst) {
    for _ in 0..b.count {
        let angle = match b.cone {
            Some((d, spread)) => d.to_angle() + rng.range(-spread, spread),
            None => rng.range(0.0, std::f32::consts::TAU),
        };
        let vel = Vec2::from_angle(angle) * rng.range(b.speed.0, b.speed.1);
        commands.spawn((
            Sprite { image: gfx.dot.clone(), custom_size: Some(Vec2::splat(b.size.0)), color: b.color.0.into(), ..default() },
            Transform::from_translation((pos + vel * 0.02).extend(z::FX + rng.f32() * 0.5)),
            Particle { vel, drag: b.drag, age: 0.0, life: rng.range(b.life.0, b.life.1), size: b.size, color: b.color },
            DespawnOnExit(Screen::Stage),
        ));
    }
}

/// Burst with no cone. Positional so effect recipes stay one line each.
fn b(count: u32, speed: (f32, f32), life: (f32, f32), size: (f32, f32), color: (LinearRgba, LinearRgba), drag: f32) -> Burst {
    Burst { count, speed, life, size, color, drag, cone: None }
}

/// Single stationary flash particle.
fn flash(life: f32, size: (f32, f32), color: (LinearRgba, LinearRgba)) -> Burst {
    b(1, (0.0, 0.0), (life, life), size, color, 1.0)
}

fn explosion(emit: &mut impl FnMut(Vec2, Burst), pos: Vec2, k: f32) {
    let n = |c: f32| (c * k) as u32;
    emit(pos, flash(0.22, (14.0 * k, 34.0 * k), (lin(6.0, 5.0, 3.0, 1.0), lin(3.0, 1.0, 0.2, 0.0))));
    emit(pos, b(n(16.0), (10.0, 45.0 * k), (0.35, 0.75), (7.0, 15.0 * k), (lin(4.0, 2.6, 0.9, 1.0), lin(0.6, 0.08, 0.02, 0.0)), 0.02));
    let smoke = (lin(0.12, 0.11, 0.1, 0.7), lin(0.05, 0.05, 0.05, 0.0));
    emit(pos, Burst { cone: Some((Vec2::Y, 1.2)), ..b(n(10.0), (4.0, 16.0), (0.9, 1.6), (6.0, 18.0 * k), smoke, 0.2) });
    emit(pos, b(n(14.0), (60.0, 140.0 * k), (0.2, 0.5), (2.2, 0.5), (lin(5.0, 3.5, 1.2, 1.0), lin(2.0, 0.4, 0.1, 0.0)), 0.05));
}

fn spawn_fx(mut commands: Commands, gfx: Res<Gfx>, mut rng: ResMut<Rng>, mut events: MessageReader<Fx>, mut shake: ResMut<Shake>) {
    let events: Vec<Fx> = events.read().copied().collect();
    let mut texts = Vec::new();
    let mut emit = |p: Vec2, burst_def: Burst| burst(&mut commands, &gfx, &mut rng, p, burst_def);
    for ev in events {
        let p = ev.pos;
        match ev.kind {
            FxKind::Muzzle(dir) => {
                emit(p, flash(0.07, (7.0, 3.0), (lin(5.0, 4.0, 2.0, 1.0), lin(3.0, 1.5, 0.5, 0.0))));
                let sparks = b(4, (40.0, 90.0), (0.06, 0.14), (2.0, 0.5), (lin(5.0, 3.5, 1.5, 1.0), lin(2.0, 0.6, 0.1, 0.0)), 0.01);
                emit(p, Burst { cone: Some((dir.vec(), 0.5)), ..sparks });
            }
            FxKind::BulletPop => emit(p, flash(0.12, (4.0, 10.0), (lin(4.0, 3.0, 1.5, 1.0), lin(1.0, 0.3, 0.1, 0.0)))),
            FxKind::Bricks => {
                emit(p, b(7, (20.0, 60.0), (0.25, 0.5), (2.2, 1.2), (lin(0.55, 0.16, 0.06, 1.0), lin(0.25, 0.07, 0.03, 0.0)), 0.02));
                emit(p, b(3, (3.0, 10.0), (0.4, 0.7), (4.0, 9.0), (lin(0.25, 0.2, 0.17, 0.5), lin(0.1, 0.1, 0.1, 0.0)), 0.3));
            }
            FxKind::Sparks => {
                emit(p, b(8, (50.0, 120.0), (0.1, 0.3), (1.8, 0.4), (lin(4.0, 4.0, 4.5, 1.0), lin(1.0, 0.8, 0.4, 0.0)), 0.02));
            }
            FxKind::TankBoom => {
                explosion(&mut emit, p, 1.0);
                shake.0 = (shake.0 + 0.35).min(1.0);
            }
            FxKind::BaseBoom => {
                explosion(&mut emit, p, 1.8);
                shake.0 = 1.0;
            }
            FxKind::Points(v) => texts.push((p, v)),
        }
    }
    for (p, v) in texts {
        commands.spawn((
            Text2d::new(v.to_string()),
            TextFont::from_font_size(48.0),
            TextColor(Color::srgb(1.0, 0.95, 0.8)),
            Transform::from_translation((p + Vec2::Y * 4.0).extend(z::TEXT)).with_scale(Vec3::splat(0.12)),
            Particle {
                vel: Vec2::Y * 14.0,
                drag: 0.3,
                age: 0.0,
                life: 1.0,
                size: (0.0, 0.0),
                color: (LinearRgba::WHITE, LinearRgba::WHITE),
            },
            DespawnOnExit(Screen::Stage),
        ));
    }
}

fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut parts: Query<(Entity, &mut Particle, &mut Transform, Option<&mut Sprite>, Option<&mut TextColor>, Has<Sparkle>)>,
) {
    let dt = time.delta_secs();
    for (e, mut p, mut tf, sprite, text, sparkle) in &mut parts {
        p.age += dt;
        if p.age >= p.life {
            commands.entity(e).try_despawn();
            continue;
        }
        let t = p.age / p.life;
        let drag = p.drag.powf(dt);
        p.vel *= drag;
        tf.translation += (p.vel * dt).extend(0.0);
        if sparkle {
            tf.scale = Vec3::splat(0.45 + 0.55 * (p.age * 14.0).sin().abs());
            tf.rotation = Quat::from_rotation_z(p.age * 5.0);
        } else if let Some(mut s) = sprite {
            s.custom_size = Some(Vec2::splat(p.size.0 + (p.size.1 - p.size.0) * t));
            s.color = lerp(p.color.0, p.color.1, t).into();
        }
        if let Some(mut c) = text {
            c.0 = c.0.with_alpha(1.0 - t * t);
        }
    }
}

fn shake_camera(
    time: Res<Time>,
    settings: Res<crate::settings::Settings>,
    mut shake: ResMut<Shake>,
    mut rng: ResMut<Rng>,
    mut cam: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(mut tf) = cam.single_mut() else { return };
    shake.0 = (shake.0 - time.delta_secs() * 1.6).max(0.0);
    let k = if settings.shake { shake.0 * shake.0 * 3.5 } else { 0.0 };
    tf.translation.x = 104.0 + rng.range(-k, k);
    tf.translation.y = 104.0 + rng.range(-k, k);
}
