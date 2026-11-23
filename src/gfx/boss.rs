//! Warnings for the district boss's attacks (`game::boss`): its row and column glow red while it charges a
//! barrage, and a red square pulses where a sentry is arriving.

use super::z;
use crate::game::boss::BAND;
use crate::game::{Enemy, Screen, Sim, Spawning, Tank, View};
use crate::geom::FIELD;
use crate::rules::{BOSS_CHARGE, Rank};
use bevy::prelude::*;

#[derive(Component)]
struct Warn;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, warn.run_if(in_state(Screen::Stage)).after(Sim));
}

// ponytail: respawns its few sprites every frame; keep them alive per boss if it ever shows in a profile.
fn warn(
    mut commands: Commands,
    time: Res<Time>,
    view: Res<View>,
    old: Query<Entity, With<Warn>>,
    enemies: Query<(&Transform, &Enemy, Has<Spawning>), With<Tank>>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let pulse = 0.5 + 0.5 * (time.elapsed_secs() * 14.0).sin();
    let mut put = |pos: Vec2, size: Vec2, alpha: f32| {
        commands.spawn((
            Sprite { color: Color::srgba(1.0, 0.18, 0.12, alpha), custom_size: Some(size), ..default() },
            Transform::from_translation(pos.extend(z::TANK - 0.5)),
            Warn,
            DespawnOnExit(Screen::Stage),
        ));
    };
    for (tf, enemy, spawning) in &enemies {
        let p = tf.translation.xy();
        match enemy.rank {
            Rank::Boss if enemy.skill < 0.0 => {
                // Brighter as the barrage nears.
                let a = (0.18 + 0.24 * (-enemy.skill / BOSS_CHARGE).min(1.0)) * (0.7 + 0.3 * pulse);
                put(Vec2::new(FIELD / 2.0, p.y), Vec2::new(FIELD, BAND), a);
                put(Vec2::new(p.x, view.bottom + FIELD / 2.0), Vec2::new(BAND, FIELD), a);
            }
            Rank::Sentry if spawning => put(p, Vec2::splat(18.0), 0.25 + 0.4 * pulse),
            _ => {}
        }
    }
}
