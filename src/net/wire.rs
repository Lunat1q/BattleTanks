//! Byte formats for LAN play: a packed `Intent`, the lockstep datagram, TCP frames and the discovery beacon.
//! Little-endian, no versioning beyond `PROTO`: both sides must run the same build anyway.

use crate::geom::Dir;
use crate::input::Intent;
use bevy::math::Vec2;

/// Bumped on any change to these formats or to what the simulation does per tick.
pub const PROTO: u32 = 2;
/// UDP port the host's beacon goes to; joiners listen here.
pub const BEACON_PORT: u16 = 47474;
/// Set on the host's input for a tick after which both sides load the host's snapshot.
pub const RESYNC: u16 = 1 << 15;

/// TCP frame kinds.
pub const HELLO: u8 = 1;
pub const SNAPSHOT: u8 = 2;
pub const BYE: u8 = 3;

#[derive(Default)]
pub struct W(pub Vec<u8>);

impl W {
    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.0.push(v);
        self
    }
    pub fn u16(&mut self, v: u16) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    /// Exact bits, so a hash sees every difference.
    pub fn f32(&mut self, v: f32) -> &mut Self {
        self.u32(v.to_bits())
    }
    pub fn bool(&mut self, v: bool) -> &mut Self {
        self.u8(v as u8)
    }
    pub fn usize(&mut self, v: usize) -> &mut Self {
        self.u32(v as u32)
    }
    pub fn vec2(&mut self, v: Vec2) -> &mut Self {
        self.f32(v.x).f32(v.y)
    }
    pub fn str(&mut self, s: &str) -> &mut Self {
        let b = &s.as_bytes()[..s.len().min(255)];
        self.u8(b.len() as u8);
        self.0.extend_from_slice(b);
        self
    }
}

pub struct R<'a>(pub &'a [u8]);

impl R<'_> {
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let (head, rest) = self.0.split_first_chunk::<N>()?;
        self.0 = rest;
        Some(*head)
    }
    pub fn u8(&mut self) -> Option<u8> {
        self.take::<1>().map(|b| b[0])
    }
    pub fn u16(&mut self) -> Option<u16> {
        self.take().map(u16::from_le_bytes)
    }
    pub fn u32(&mut self) -> Option<u32> {
        self.take().map(u32::from_le_bytes)
    }
    pub fn u64(&mut self) -> Option<u64> {
        self.take().map(u64::from_le_bytes)
    }
    pub fn f32(&mut self) -> Option<f32> {
        self.u32().map(f32::from_bits)
    }
    pub fn bool(&mut self) -> Option<bool> {
        self.u8().map(|b| b != 0)
    }
    pub fn usize(&mut self) -> Option<usize> {
        self.u32().map(|v| v as usize)
    }
    pub fn vec2(&mut self) -> Option<Vec2> {
        Some(Vec2::new(self.f32()?, self.f32()?))
    }
    pub fn str(&mut self) -> Option<String> {
        let n = self.u8()? as usize;
        let (s, rest) = self.0.split_at_checked(n)?;
        self.0 = rest;
        Some(String::from_utf8_lossy(s).into_owned())
    }
    /// Item `i` of `all`, or `None` for a byte that names nothing.
    pub fn pick<T: Copy>(&mut self, all: &[T]) -> Option<T> {
        all.get(self.u8()? as usize).copied()
    }
}

/// Index of `v` in `all` (the enum's declared order), as a byte.
pub fn index<T: PartialEq>(all: &[T], v: &T) -> u8 {
    all.iter().position(|x| x == v).unwrap_or(0) as u8
}

fn dir_bits(d: Option<Dir>) -> u16 {
    d.map_or(0, |d| 1 + index(&Dir::ALL, &d) as u16)
}

fn bits_dir(b: u16) -> Option<Dir> {
    Dir::ALL.get((b as usize).checked_sub(1)?).copied()
}

/// One player's intent for one tick in 13 bits (bit 15 is left for `RESYNC`).
pub fn pack(i: &Intent) -> u16 {
    let flags = [i.fire, i.fire_pressed, i.confirm, i.back, i.start, i.pause, i.away];
    flags.iter().enumerate().fold(dir_bits(i.dir) | dir_bits(i.nav) << 3, |acc, (n, &f)| acc | (f as u16) << (6 + n))
}

pub fn unpack(b: u16) -> Intent {
    let f = |n: u16| b >> (6 + n) & 1 != 0;
    Intent {
        dir: bits_dir(b & 7),
        nav: bits_dir(b >> 3 & 7),
        fire: f(0),
        fire_pressed: f(1),
        confirm: f(2),
        back: f(3),
        start: f(4),
        pause: f(5),
        away: f(6),
    }
}

/// Held state carries over; one-shot presses since the last packed tick add up, so a press made on a frame
/// that ran no tick still lands on the next one.
pub fn gather(pending: Intent, now: Intent) -> Intent {
    Intent { dir: now.dir, fire: now.fire, ..crate::input::merge(now, pending) }
}

/// What is left of `sent` for the following tick: held state only.
pub fn held(sent: Intent) -> Intent {
    Intent { dir: sent.dir, fire: sent.fire, ..Intent::default() }
}

/// The lockstep datagram, sent every frame both ways: our inputs from tick `first` on (resent until acked, so a
/// lost packet costs nothing), the next tick we still need from the peer, our latest state hash, and a clock stamp
/// plus the echo of the peer's latest one (with how long we sat on it), which times the round trip.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Packet {
    pub session: u32,
    pub first: u32,
    pub inputs: Vec<u16>,
    pub ack: u32,
    pub hash: Option<(u32, u64)>,
    /// Sender's clock, ms.
    pub stamp: u32,
    /// The peer's latest `stamp` we got, and ms since we got it.
    pub echo: Option<(u32, u16)>,
}

impl Packet {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = W::default();
        w.u8(b'B').u32(self.session).u32(self.first).u8(self.inputs.len().min(255) as u8);
        for &i in self.inputs.iter().take(255) {
            w.u16(i);
        }
        w.u32(self.ack);
        let (t, h) = self.hash.unwrap_or((u32::MAX, 0));
        w.u32(t).u64(h).u32(self.stamp);
        let (e, age) = self.echo.unwrap_or((u32::MAX, 0));
        w.u32(e).u16(age);
        w.0
    }

    pub fn decode(b: &[u8]) -> Option<Packet> {
        let mut r = R(b);
        (r.u8()? == b'B').then_some(())?;
        let (session, first, n) = (r.u32()?, r.u32()?, r.u8()?);
        let inputs = (0..n).map(|_| r.u16()).collect::<Option<Vec<_>>>()?;
        let ack = r.u32()?;
        let (t, h, stamp) = (r.u32()?, r.u64()?, r.u32()?);
        let (e, age) = (r.u32()?, r.u16()?);
        Some(Packet {
            session,
            first,
            inputs,
            ack,
            hash: (t != u32::MAX).then_some((t, h)),
            stamp,
            echo: (e != u32::MAX).then_some((e, age)),
        })
    }
}

/// The host's beacon: `BTANKS <proto> <version> <tcp port> <name>`.
pub fn beacon(version: &str, port: u16, name: &str) -> Vec<u8> {
    format!("BTANKS {PROTO} {version} {port} {name}").into_bytes()
}

/// `(version, tcp port, name)` from a beacon of our protocol.
pub fn parse_beacon(b: &[u8]) -> Option<(String, u16, String)> {
    let s = std::str::from_utf8(b).ok()?;
    let mut parts = s.splitn(5, ' ');
    (parts.next()? == "BTANKS" && parts.next()?.parse::<u32>().ok()? == PROTO).then_some(())?;
    let version = parts.next()?.to_string();
    let port = parts.next()?.parse().ok()?;
    let name = parts.next().unwrap_or("?").trim().to_string();
    Some((version, port, name))
}

/// FNV-1a: tiny and stable everywhere (std's hasher is randomised per process).
pub fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::default;

    #[test]
    fn intents_survive_packing() {
        let samples = [
            Intent::default(),
            Intent { dir: Some(Dir::Left), fire: true, confirm: true, ..default() },
            Intent { nav: Some(Dir::Right), dir: Some(Dir::Up), back: true, start: true, pause: true, away: true, ..default() },
            Intent { dir: Some(Dir::Down), fire_pressed: true, ..default() },
        ];
        for i in samples {
            assert_eq!(unpack(pack(&i)), i);
            assert_eq!(pack(&i) & RESYNC, 0);
            assert_eq!(unpack(pack(&i) | RESYNC), i, "the resync flag rides along");
        }
    }

    #[test]
    fn presses_between_ticks_are_not_lost() {
        let tap = Intent { confirm: true, nav: Some(Dir::Up), ..default() };
        let later = gather(tap, Intent { dir: Some(Dir::Left), ..default() });
        assert!(later.confirm && later.nav == Some(Dir::Up), "the press waits for the next tick");
        assert_eq!(later.dir, Some(Dir::Left), "held state is the latest");
        assert_eq!(held(later), Intent { dir: Some(Dir::Left), ..default() }, "a second tick in one frame gets no repeat press");
    }

    #[test]
    fn packets_round_trip() {
        let p = Packet {
            session: 9,
            first: 120,
            inputs: vec![1, 2, RESYNC | 3],
            ack: 118,
            hash: Some((90, 0xDEAD_BEEF)),
            stamp: 5000,
            echo: Some((4980, 7)),
        };
        assert_eq!(Packet::decode(&p.encode()), Some(p));
        let bare = Packet { session: 1, ..default() };
        assert_eq!(Packet::decode(&bare.encode()), Some(bare));
        assert_eq!(Packet::decode(b"junk"), None);
        assert_eq!(Packet::decode(&[]), None);
    }

    #[test]
    fn beacons_parse_and_ignore_strangers() {
        assert_eq!(parse_beacon(&beacon("0.4.6", 5123, "Max's Pixel")), Some(("0.4.6".into(), 5123, "Max's Pixel".into())));
        assert_eq!(parse_beacon(b"BTANKS 999 0.4.6 5123 x"), None, "other protocol");
        assert_eq!(parse_beacon(b"HELLO"), None);
    }

    #[test]
    fn writer_and_reader_agree() {
        let mut w = W::default();
        w.u8(7).u16(300).u32(70_000).u64(u64::MAX).f32(-1.5).bool(true).vec2(Vec2::new(2.0, 3.0)).str("tank");
        let mut r = R(&w.0);
        assert_eq!((r.u8(), r.u16(), r.u32(), r.u64()), (Some(7), Some(300), Some(70_000), Some(u64::MAX)));
        assert_eq!((r.f32(), r.bool(), r.vec2(), r.str()), (Some(-1.5), Some(true), Some(Vec2::new(2.0, 3.0)), Some("tank".into())));
        assert_eq!(r.u8(), None, "nothing left");
        assert_ne!(fnv(b"a"), fnv(b"b"));
    }
}
