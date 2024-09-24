use mio::{net::UdpSocket, Events, Interest, Poll, Token, Waker};
use std::{
    hash::Hash,
    io::{ErrorKind, Read, Result as IoResult, Write},
    net::{Ipv4Addr, SocketAddr},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread::{Scope, ScopedJoinHandle},
    time::Duration,
};

pub type Subscriber = fn(message_id: u64, message: &[u8]);

const MCAST_SOCKET: Token = Token(0);
const WAKER: Token = Token(1024);
const RW_INTERESTS: Interest = Interest::READABLE.add(Interest::WRITABLE);

#[derive(Debug)]
struct Channel {
    cid: u64,
    seq: u64,
    last_send: u64,
}

impl Hash for Channel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.cid)
    }
}

impl PartialEq for Channel {
    fn eq(&self, other: &Self) -> bool {
        self.cid == other.cid
    }
}

impl Eq for Channel {}

pub fn new_peer<'scp, 'env>(
    addr: SocketAddr,
    scope: &'scp Scope<'scp, 'env>,
) -> IoResult<(
    PeerManager,
    ScopedJoinHandle<'scp, Result<(), std::io::Error>>,
)> {
    if !addr.ip().is_multicast() {
        return Err(ErrorKind::AddrInUse.into());
    }
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(16);

    // bind us to the socket address.
    let mut socket = UdpSocket::bind(addr)?;
    match addr {
        SocketAddr::V4(addr_v4) => {
            // join to the multicast address, with all interfaces
            socket.join_multicast_v4(addr_v4.ip(), &Ipv4Addr::UNSPECIFIED)?;
            socket.set_multicast_loop_v4(false)?;
        }
        SocketAddr::V6(addr_v6) => {
            // join to the multicast address, with all interfaces
            socket.join_multicast_v6(addr_v6.ip(), 0)?;
            socket.set_multicast_loop_v6(false)?;
        }
    };

    poll.registry()
        .register(&mut socket, MCAST_SOCKET, RW_INTERESTS)?;
    let waker = Waker::new(poll.registry(), WAKER)?;
    let (rcv, peer_manager) = {
        let (sx, rx) = channel();
        (rx, PeerManager::new(sx, Arc::new(waker)))
    };
    let handle = scope.spawn(move || -> IoResult<()> {
        let mut subscribers: Vec<(u16, Subscriber)> = Vec::with_capacity(128);
        let mut buf = [0; 1 << 16];
        loop {
            if let Err(err) = poll.poll(&mut events, Some(Duration::from_secs(1))) {
                if err.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            }
            for event in events.iter() {
                println!("Got {:?}", event);
                match event.token() {
                    MCAST_SOCKET => loop {
                        match socket.recv_from(&mut buf) {
                            Ok((packet_size, source_address)) => {
                                let msg = std::str::from_utf8(&buf[..packet_size]).unwrap();
                                println!("Got {} from  {}", &msg, &source_address);
                                for (_cid, subscr) in &subscribers {
                                    subscr(1, &buf[..packet_size]);
                                }
                            }
                            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                                break;
                            }
                            Err(e) => {
                                return Err(e);
                            }
                        }
                    },
                    WAKER => {
                        for cmd in rcv.try_iter() {
                            println!("Got cmd {:?}", &cmd);
                            match cmd {
                                PeerCommand::Subscribe {
                                    cid,
                                    subscriber: listener,
                                } => {
                                    subscribers.push((cid, listener));
                                }
                                PeerCommand::UnSubscribe { cid } => {
                                    subscribers.retain(|(ccid, _)| cid != *ccid)
                                }
                                PeerCommand::Publish { cid, msg } => {
                                    socket.send(&msg.pack(cid, 1)[..])?;
                                }
                                PeerCommand::Stop => return Ok(()),
                            }
                        }
                    }
                    _ => {
                        // warn!("Got event for unexpected token: {:?}", event);
                    }
                }
            }
        }
        // println!("Peer shuted down!!!!");
    });
    Ok((peer_manager, handle))
}

#[derive(Debug)]
struct ChannelState {
    id: u16,
    seq: u64,
}

//will keep  80  bytes for possible headers
const MAX_MSG_LEN: u16 = 65399;

#[derive(Debug)]
#[repr(transparent)]
pub struct Message {
    data: Vec<u8>,
}

impl Message {
    pub fn new(len: u16) -> Self {
        assert!(len <= MAX_MSG_LEN);
        //eventually header may be a type param to this class
        //reserve 10 bytes for header, 2 for cid, and 8 for cid sequence
        let mut data = Vec::with_capacity(len as usize + 2 + 8);
        for _ in 0..10 {
            data.push(0u8);
        }
        Message { data }
    }

    pub(crate) fn pack(mut self, cid: u16, cseq: u64) -> Vec<u8> {
        let cid_b = cid.to_le_bytes();
        self.data[0..2].copy_from_slice(&cid_b[..]);
        let cseq_b = cseq.to_le_bytes();
        self.data[2..10].copy_from_slice(&cseq_b[..]);
        self.data
    }
}

impl Write for Message {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.data.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

#[derive(Debug)]
enum PeerCommand {
    Subscribe { cid: u16, subscriber: Subscriber },
    UnSubscribe { cid: u16 },
    Publish { cid: u16, msg: Message },
    Stop,
}

#[derive(Debug, Clone)]
pub struct PeerManager {
    cmd_snd: Sender<PeerCommand>,
    waker: Arc<Waker>,
}

impl PeerManager {
    #[inline]
    fn new(cmd_snd: Sender<PeerCommand>, waker: Arc<Waker>) -> Self {
        PeerManager { cmd_snd, waker }
    }

    #[inline]
    fn wakeup(&mut self, cmd: PeerCommand) -> IoResult<()> {
        self.cmd_snd.send(cmd).unwrap();
        self.waker.wake()
    }

    #[inline]
    pub fn subscribe(&mut self, cid: u16, subscriber: Subscriber) -> IoResult<()> {
        self.wakeup(PeerCommand::Subscribe { cid, subscriber })
    }

    #[inline]
    pub fn unsubscribe(&mut self, cid: u16) -> IoResult<()> {
        self.wakeup(PeerCommand::UnSubscribe { cid })
    }

    #[inline]
    pub fn shutdown(&mut self) -> IoResult<()> {
        self.wakeup(PeerCommand::Stop)
    }

    pub fn publish(&mut self, cid: u16, msg: Message) -> IoResult<()> {
        self.wakeup(PeerCommand::Publish { cid, msg })
    }
}
