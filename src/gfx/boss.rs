//! Warnings for the district boss's attacks (`game::boss`): its row and column glow red while it charges a
//! barrage (and until its echo), so do the row and column it marked through a player, and a red square pulses where
//! a sentry is arriving. Bosses and mini-bosses carry an HP bar under them.

use super::z;
use crate::game::boss::{BAND, glow};
use crate::game::{Enemy, Screen, Session, Sim, Spawning, Tank, View};
use crate::geom::FIELD;
use crate::geom::SUB;
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
    enemies: Query<(&Transform, &Enemy, &Tank, Has<Spawning>)>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let pulse = 0.5 + 0.5 * (time.elapsed_secs() * 14.0).sin();
    let mut sprite = |pos: Vec2, size: Vec2, color: Color, z: f32| {
        commands.spawn((
            Sprite { color, custom_size: Some(size), ..default() },
            Transform::from_translation(pos.extend(z)),
            Warn,
            DespawnOnExit(Screen::Stage),
        ));
    };
    for (tf, enemy, tank, spawning) in &enemies {
        let p = tf.translation.xy();
        if matches!(enemy.rank, Rank::Boss | Rank::Mini) && !spawning {
            // Over the trees so it reads in a forest; under the tank, where the map's top edge can't hide it.
            let (w, y) = (tank.size, p.y - tank.size / 2.0 - SUB);
            sprite(Vec2::new(p.x, y), Vec2::new(w + 2.0, SUB), Color::srgba(0.0, 0.0, 0.0, 0.75), z::TREES + 0.5);
            let fill = bar_width(enemy.hp, enemy.max_hp, w);
            let red = Color::srgb(1.0, 0.25, 0.2);
            sprite(Vec2::new(p.x - (w - fill) / 2.0, y), Vec2::new(fill, SUB - 2.0), red, z::TREES + 0.6);
        }
        let mut put = |pos: Vec2, size: Vec2, alpha: f32| sprite(pos, size, Color::srgba(1.0, 0.18, 0.12, alpha), z::TANK - 0.5);
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

/// Filled width of an HP bar `width` wide: never less than one unit while the tank lives, so the last hit shows.
fn bar_width(hp: u32, max_hp: u32, width: f32) -> f32 {
    if hp == 0 { 0.0 } else { (width * hp as f32 / max_hp.max(1) as f32).max(1.0) }
}

#[cfg(test)]
mod tests {
    use super::bar_width;

    #[test]
    fn hp_bar_shrinks_with_the_hp_and_keeps_a_sliver() {
        assert_eq!(bar_width(50, 50, 32.0), 32.0);
        assert_eq!(bar_width(25, 50, 32.0), 16.0);
        assert_eq!(bar_width(1, 500, 32.0), 1.0);
        assert_eq!(bar_width(0, 50, 32.0), 0.0);
    }
}
