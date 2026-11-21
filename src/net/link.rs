//! Sockets for LAN play, std only: the host's beacon + TCP listener, the joiner's beacon listener, and a linked
//! pair (TCP for hello / snapshots / goodbye with a reader thread, UDP for the lockstep datagrams).

use super::wire::{self, BEACON_PORT, BYE, HELLO, PROTO, R, W};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};

/// A host heard on the LAN.
pub struct Found {
    pub name: String,
    pub version: String,
    pub addr: SocketAddr,
    seen: Instant,
}

/// Joiner side: collects beacons.
pub struct Finder {
    sock: UdpSocket,
    pub found: Vec<Found>,
}

impl Finder {
    pub fn new() -> io::Result<Self> {
        let sock = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, BEACON_PORT))?;
        sock.set_nonblocking(true)?;
        Ok(Finder { sock, found: Vec::new() })
    }

    pub fn poll(&mut self) {
        let mut buf = [0u8; 512];
        while let Ok((n, from)) = self.sock.recv_from(&mut buf) {
            let Some((version, port, name)) = wire::parse_beacon(&buf[..n]) else { continue };
            let addr = SocketAddr::new(from.ip(), port);
            match self.found.iter_mut().find(|f| f.addr == addr) {
                Some(f) => (f.seen, f.name, f.version) = (Instant::now(), name, version),
                None => self.found.push(Found { name, version, addr, seen: Instant::now() }),
            }
        }
        self.found.retain(|f| f.seen.elapsed() < Duration::from_secs(3));
    }
}

/// Host side until someone connects: a beacon a second, and the TCP port it names.
pub struct Beacon {
    sock: UdpSocket,
    listener: TcpListener,
    msg: Vec<u8>,
    last: Option<Instant>,
    targets: Vec<SocketAddr>,
}

impl Beacon {
    pub fn new(version: &str, name: &str) -> io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        listener.set_nonblocking(true)?;
        let sock = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        sock.set_broadcast(true)?;
        let msg = wire::beacon(version, listener.local_addr()?.port(), name);
        Ok(Beacon { sock, listener, msg, last: None, targets: broadcast_targets() })
    }

    /// Sends the beacon when due; a connecting joiner, if any.
    pub fn poll(&mut self) -> Option<TcpStream> {
        if self.last.is_none_or(|t| t.elapsed() >= Duration::from_secs(1)) {
            self.last = Some(Instant::now());
            for t in &self.targets {
                let _ = self.sock.send_to(&self.msg, t); // an unreachable target just doesn't hear it
            }
        }
        self.listener.accept().ok().map(|(s, _)| s)
    }
}

/// Everywhere a beacon goes: the all-hosts broadcast, the /24 broadcast of the address that routes outwards (some
/// Wi-Fi drops 255.255.255.255), and loopback so two copies on one PC find each other.
/// ponytail: a /24 guess, no interface list (std has none); a phone hotspot host may need the platform's list.
fn broadcast_targets() -> Vec<SocketAddr> {
    let mut t = vec![SocketAddr::from(([255, 255, 255, 255], BEACON_PORT))];
    if let Some(IpAddr::V4(ip)) = local_ip() {
        let o = ip.octets();
        t.push(SocketAddr::from(([o[0], o[1], o[2], 255], BEACON_PORT)));
    }
    t.push(SocketAddr::from((Ipv4Addr::LOCALHOST, BEACON_PORT)));
    t
}

/// The local address the OS would route outwards with (no packet is sent).
fn local_ip() -> Option<IpAddr> {
    let s = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    s.connect(("8.8.8.8", 80)).ok()?;
    s.local_addr().ok().map(|a| a.ip())
}

/// What each side says first. The host's copy also carries the session's shared starting state.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Hello {
    pub version: String,
    pub name: String,
    pub udp: u16,
    /// Tags the session's datagrams.
    pub session: u32,
    pub seed: u64,
    pub friendly_fire: bool,
    pub keep_power: bool,
    pub unlocked: u8,
    pub classic: u8,
}

impl Hello {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = W::default();
        w.u32(PROTO).str(&self.version).str(&self.name).u16(self.udp).u32(self.session).u64(self.seed);
        w.bool(self.friendly_fire).bool(self.keep_power).u8(self.unlocked).u8(self.classic);
        w.0
    }

    /// `None` for garbage or another protocol.
    pub fn decode(b: &[u8]) -> Option<Hello> {
        let mut r = R(b);
        (r.u32()? == PROTO).then_some(())?;
        Some(Hello {
            version: r.str()?,
            name: r.str()?,
            udp: r.u16()?,
            session: r.u32()?,
            seed: r.u64()?,
            friendly_fire: r.bool()?,
            keep_power: r.bool()?,
            unlocked: r.u8()?,
            classic: r.u8()?,
        })
    }
}

/// A connected pair of machines.
pub struct Conn {
    tcp: TcpStream,
    /// Behind a lock only because Bevy resources must be `Sync`; one thread reads it.
    rx: Mutex<mpsc::Receiver<(u8, Vec<u8>)>>,
    udp: UdpSocket,
    /// The peer's lockstep socket, once its hello named the port.
    pub peer: Option<SocketAddr>,
}

impl Conn {
    pub fn new(tcp: TcpStream) -> io::Result<Self> {
        tcp.set_nonblocking(false)?;
        tcp.set_nodelay(true)?;
        tcp.set_write_timeout(Some(Duration::from_secs(5)))?;
        let reader = tcp.try_clone()?;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || read_frames(reader, tx));
        let udp = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        udp.set_nonblocking(true)?;
        Ok(Conn { tcp, rx: Mutex::new(rx), udp, peer: None })
    }

    pub fn connect(addr: SocketAddr) -> io::Result<Self> {
        Conn::new(TcpStream::connect_timeout(&addr, Duration::from_secs(3))?)
    }

    pub fn udp_port(&self) -> u16 {
        self.udp.local_addr().map_or(0, |a| a.port())
    }

    /// The peer said hello: aim our datagrams at its port.
    pub fn greeted(&mut self, hello: &Hello) {
        self.peer = self.tcp.peer_addr().ok().map(|a| SocketAddr::new(a.ip(), hello.udp));
    }

    /// One frame: `len (u32, incl. kind) | kind | payload`. False if the link is gone.
    pub fn send(&mut self, kind: u8, payload: &[u8]) -> bool {
        let mut frame = Vec::with_capacity(payload.len() + 5);
        frame.extend_from_slice(&(payload.len() as u32 + 1).to_le_bytes());
        frame.push(kind);
        frame.extend_from_slice(payload);
        self.tcp.write_all(&frame).is_ok()
    }

    pub fn hello(&mut self, hello: &Hello) -> bool {
        self.send(HELLO, &hello.encode())
    }

    /// The next frame from the peer; `BYE` once the link closed.
    pub fn recv(&self) -> Option<(u8, Vec<u8>)> {
        match self.rx.lock().map_or(Err(mpsc::TryRecvError::Disconnected), |rx| rx.try_recv()) {
            Ok(f) => Some(f),
            Err(mpsc::TryRecvError::Disconnected) => Some((BYE, Vec::new())),
            Err(mpsc::TryRecvError::Empty) => None,
        }
    }

    pub fn send_udp(&self, bytes: &[u8]) {
        if let Some(p) = self.peer {
            let _ = self.udp.send_to(bytes, p); // lost datagrams are resent by design
        }
    }

    /// The next datagram from the peer's address (strays are dropped).
    pub fn recv_udp(&self, buf: &mut [u8]) -> Option<usize> {
        loop {
            let (n, from) = self.udp.recv_from(buf).ok()?;
            if self.peer.is_some_and(|p| p.ip() == from.ip()) {
                return Some(n);
            }
        }
    }
}

impl Drop for Conn {
    fn drop(&mut self) {
        let _ = self.tcp.shutdown(std::net::Shutdown::Both); // also ends the reader thread
    }
}

fn read_frames(mut s: TcpStream, tx: mpsc::Sender<(u8, Vec<u8>)>) {
    loop {
        let mut len = [0u8; 4];
        if s.read_exact(&mut len).is_err() {
            break;
        }
        let n = u32::from_le_bytes(len) as usize;
        if n == 0 || n > 16 << 20 {
            break;
        }
        let mut buf = vec![0u8; n];
        if s.read_exact(&mut buf).is_err() || tx.send((buf[0], buf[1..].to_vec())).is_err() {
            break;
        }
    }
    let _ = tx.send((BYE, Vec::new()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_round_trips() {
        let h = Hello {
            version: "0.4.6".into(),
            name: "Pixel".into(),
            udp: 5000,
            session: 7,
            seed: 99,
            friendly_fire: true,
            keep_power: false,
            unlocked: 4,
            classic: 12,
        };
        assert_eq!(Hello::decode(&h.encode()), Some(h));
        assert_eq!(Hello::decode(&[9, 9, 9]), None);
    }

    /// Retries `f` for up to a second.
    fn soon<T>(mut f: impl FnMut() -> Option<T>) -> Option<T> {
        (0..200).find_map(|_| {
            f().or_else(|| {
                std::thread::sleep(Duration::from_millis(5));
                None
            })
        })
    }

    #[test]
    fn two_ends_talk_over_loopback() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let mut a = Conn::connect(listener.local_addr().unwrap()).unwrap();
        let mut b = Conn::new(listener.accept().unwrap().0).unwrap();
        let ha = Hello { udp: a.udp_port(), ..Hello::default() };
        let hb = Hello { udp: b.udp_port(), ..Hello::default() };
        assert!(a.hello(&ha) && b.hello(&hb));
        let (kind, body) = soon(|| b.recv()).unwrap();
        assert_eq!((kind, Hello::decode(&body)), (HELLO, Some(ha.clone())));
        b.greeted(&ha);
        a.greeted(&hb);
        a.send_udp(b"tick");
        let mut buf = [0u8; 16];
        let n = soon(|| b.recv_udp(&mut buf)).unwrap();
        assert_eq!(&buf[..n], b"tick");
        drop(a);
        assert_eq!(soon(|| b.recv()).map(|f| f.0), Some(BYE), "a hung up");
    }
}
