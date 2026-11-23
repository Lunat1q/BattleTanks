//! The VERSUS part of a stage snapshot (`snap`): the match (round wins change on the stage) and both NPC armies.

use super::wire::{R, W};
use crate::game::versus::{Armies, Army, Match};
use crate::rules::EnemyKind;

fn put_kinds(w: &mut W, kinds: &[EnemyKind]) {
    w.usize(kinds.len());
    kinds.iter().for_each(|k| _ = w.u8(k.index() as u8));
}

fn get_kinds(r: &mut R) -> Option<Vec<EnemyKind>> {
    (0..r.usize()?).map(|_| r.pick(&EnemyKind::ALL)).collect()
}

pub fn put(w: &mut W, m: &Match, a: &Armies) {
    w.u32(m.wins[0]).u32(m.wins[1]).usize(m.round).u8(m.last.map_or(255, |p| p as u8)).u32(m.budget).u32(m.rounds).usize(m.map);
    m.armies.iter().for_each(|k| put_kinds(w, k));
    for s in &a.sides {
        put_kinds(w, &s.roster);
        w.usize(s.next).f32(s.timer).usize(s.point);
    }
    w.f32(a.drop_in);
}

pub fn get(r: &mut R) -> Option<(Match, Armies)> {
    let (wins, round, last) = ([r.u32()?, r.u32()?], r.usize()?, r.u8()?);
    let (budget, rounds, map) = (r.u32()?, r.u32()?, r.usize()?);
    let armies = [get_kinds(r)?, get_kinds(r)?];
    let last = (last != 255).then_some(last as usize);
    let duel = Match { wins, round, last, budget, rounds, map, armies };
    let mut army = || Some(Army { roster: get_kinds(r)?, next: r.usize()?, timer: r.f32()?, point: r.usize()? });
    let sides = [army()?, army()?];
    Some((duel, Armies { sides, drop_in: r.f32()? }))
}
