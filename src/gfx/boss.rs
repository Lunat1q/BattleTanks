//! Warnings for the district boss's attacks (`game::boss`): its row and column glow red while it charges a
//! barrage (and until its echo), so do the row and column it marked through a player, and a red square pulses where
//! a sentry is arriving.

use super::z;
use crate::game::boss::{BAND, glow};
use crate::game::{Enemy, Screen, Session, Sim, Spawning, Tank, View};
use crate::geom::FIELD;
use crate::rules::{self, Rank};
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
    session: Res<Session>,
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
            Rank::Boss => {
                let kit = rules::boss_kit(session.stage);
                let gap = rules::boss_barrage_gap(session.stage, kit.rage && enemy.phase >= 3);
                let Some(g) = glow(enemy.skill, gap, kit.echo) else { continue };
                // Brighter as the barrage nears.
                let a = (0.18 + 0.24 * g) * (0.7 + 0.3 * pulse);
                for m in [Some(p), enemy.mark].into_iter().flatten() {
                    put(Vec2::new(FIELD / 2.0, m.y), Vec2::new(FIELD, BAND), a);
                    put(Vec2::new(m.x, view.bottom + FIELD / 2.0), Vec2::new(BAND, FIELD), a);
                }
            }
            Rank::Sentry if spawning => put(p, Vec2::splat(18.0), 0.25 + 0.4 * pulse),
            _ => {}
        }
    }
}
