use crate::config;
use trust_dns_resolver::Resolver;
use std::{io::prelude::*, net::{TcpListener, Shutdown}};

use wayland_client::protocol::{
    wl_pointer::{Axis, ButtonState},
    wl_keyboard::KeyState,
};

use std::net::{SocketAddr, UdpSocket, TcpStream};

pub trait Resolve {
    fn resolve(&self) -> Option<SocketAddr>;
}

impl Resolve for Option<config::Client> {
    fn resolve(&self) -> Option<SocketAddr> {
        if let Some(client) = self {
            let ip = if let Some(ip) = client.ip {
                ip
            } else {
                match &client.host_name {
                    Some(host) => {
                        let resolver = Resolver::from_system_conf().unwrap();
                        let response = resolver
                            .lookup_ip(host.as_str())
                            .expect(format!("couldn't resolve {}", host.as_str()).as_str());
                        if let Some(ip) = response.iter().next() {
                            ip
                        } else {
                            panic!("couldn't resolve host: {}", host)
                        }
                    }
                    None => {
                        panic!("either ip or host_name must be specified");
                    }
                }
            };
            let port = if let Some(port) = client.port { port } else { 42069 };
            Some(SocketAddr::new(ip, port))
        } else {
            None
        }
    }
}

struct ClientAddrs {
    _left: Option<SocketAddr>,
    right: Option<SocketAddr>,
    _top: Option<SocketAddr>,
    _bottom: Option<SocketAddr>,
}

pub struct Connection {
    udp_socket: UdpSocket,
    port: u16,
    client: ClientAddrs,
}

pub enum Event {
    Mouse{t: u32, x: f64, y: f64},
    Button{t: u32, b: u32, s: ButtonState},
    Axis{t: u32, a: Axis, v: f64},
    Key{t: u32, k: u32, s: KeyState},
    KeyModifier{mods_depressed: u32, mods_latched: u32, mods_locked: u32, group: u32},
}

impl Connection {
    pub fn new(config: config::Config) -> Connection {
        let clients = ClientAddrs {
            _left: config.client.left.resolve(),
            right: config.client.right.resolve(),
            _top: config.client.top.resolve(),
            _bottom: config.client.bottom.resolve(),
        };
        Connection {
            udp_socket: UdpSocket::bind(SocketAddr::new("0.0.0.0".parse().unwrap(), config.port.unwrap_or(42069)))
                .unwrap(),
            port: if let Some(port) = config.port { port } else { 42069 },
            client: clients,
        }
    }


    pub fn send_data(&self, buf: &[u8]) {
        if let Some(addr) = self.client.right {
            let addr = SocketAddr::new(addr.ip(), 42069);
            let mut stream = TcpStream::connect(addr).unwrap();
            println!("sending {} bytes!", buf.len());
            stream.write(&buf.len().to_ne_bytes()).unwrap();
            stream.write(buf).unwrap();
            stream.flush().unwrap();
        }
    }

    pub fn receive_data(&self) -> Vec<u8> {
        let sock = TcpListener::bind(SocketAddr::new("0.0.0.0".parse().unwrap(), 42069)).unwrap();
        let (mut client_sock, addr) = sock.accept().unwrap();
        println!("receiving from {}", addr);
        let mut buf = [0u8;8];
        client_sock.read_exact(&mut buf[..]).unwrap();
        let len = usize::from_ne_bytes(buf);
        let mut data: Vec<u8> = Vec::with_capacity(len as usize);
        client_sock.read_exact(&mut data[..]).unwrap();
        data
    }

    pub fn send_event(&self, e: &Event) {
        // TODO check which client
        if let Some(addr) = self.client.right {
            let buf = e.encode();
            self.udp_socket.send_to(&buf, addr).unwrap();
        }
    }

    pub fn receive_event(&self) -> Option<Event> {
        let mut buf = [0u8; 21];
        if let Ok((_amt, _src)) = self.udp_socket.recv_from(&mut buf) {
            Some(Event::decode(buf))
        } else {
            None
        }
    }
}

impl Event {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Event::Mouse { t, x, y } => {
                let mut buf = Vec::new();
                buf.push(0u8);
                buf.extend_from_slice(t.to_ne_bytes().as_ref());
                buf.extend_from_slice(x.to_ne_bytes().as_ref());
                buf.extend_from_slice(y.to_ne_bytes().as_ref());
                buf
            }
            Event::Button { t, b, s } => {
                let mut buf = Vec::new();
                buf.push(1u8);
                buf.extend_from_slice(t.to_ne_bytes().as_ref());
                buf.extend_from_slice(b.to_ne_bytes().as_ref());
                buf.push(match s {
                    ButtonState::Released => 0u8, 
                    ButtonState::Pressed => 1u8, 
                    _ => todo!()
                });
                buf
            }
            Event::Axis{t, a, v} => {
                let mut buf = Vec::new();
                buf.push(2u8);
                buf.extend_from_slice(t.to_ne_bytes().as_ref());
                buf.push(match a {
                    Axis::VerticalScroll => 0,
                    Axis::HorizontalScroll => 1,
                    _ => todo!()
                });
                buf.extend_from_slice(v.to_ne_bytes().as_ref());
                buf
            }
            Event::Key{t, k, s } => {
                let mut buf = Vec::new();
                buf.push(3u8);
                buf.extend_from_slice(t.to_ne_bytes().as_ref());
                buf.extend_from_slice(k.to_ne_bytes().as_ref());
                buf.push(match s {
                    KeyState::Released => 0, 
                    KeyState::Pressed => 1, 
                    _ => todo!(),
                });
                buf
            }
            Event::KeyModifier{ mods_depressed, mods_latched, mods_locked, group } => {
                let mut buf = Vec::new();
                buf.push(4u8);
                buf.extend_from_slice(mods_depressed.to_ne_bytes().as_ref());
                buf.extend_from_slice(mods_latched.to_ne_bytes().as_ref());
                buf.extend_from_slice(mods_locked.to_ne_bytes().as_ref());
                buf.extend_from_slice(group.to_ne_bytes().as_ref());
                buf
            }
        }
    }

    pub fn decode(buf: [u8; 21]) -> Event {
        match buf[0] {
            0 => Self::Mouse {
                t: u32::from_ne_bytes(buf[1..5].try_into().unwrap()),
                x: f64::from_ne_bytes(buf[5..13].try_into().unwrap()),
                y: f64::from_ne_bytes(buf[13..21].try_into().unwrap()),
            },
            1 => Self::Button {
                t: (u32::from_ne_bytes(buf[1..5].try_into().unwrap())),
                b: (u32::from_ne_bytes(buf[5..9].try_into().unwrap())),
                s: (match buf[9] {
                    0 => ButtonState::Released,
                    1 => ButtonState::Pressed,
                    _ => panic!("protocol violation")
                })
            },
            2 => Self::Axis {
                t: (u32::from_ne_bytes(buf[1..5].try_into().unwrap())),
                a: (match buf[5] {
                    0 => Axis::VerticalScroll,
                    1 => Axis::HorizontalScroll,
                    _ => todo!()
                }),
                v: (f64::from_ne_bytes(buf[6..14].try_into().unwrap())),
            },
            3 => Self::Key {
                t: u32::from_ne_bytes(buf[1..5].try_into().unwrap()),
                k: u32::from_ne_bytes(buf[5..9].try_into().unwrap()),
                s: match buf[9] {
                    0 => KeyState::Released,
                    1 => KeyState::Pressed,
                    _ => todo!(),
                }
            },
            4 => Self::KeyModifier {
                mods_depressed: u32::from_ne_bytes(buf[1..5].try_into().unwrap()),
                mods_latched: u32::from_ne_bytes(buf[5..9].try_into().unwrap()),
                mods_locked: u32::from_ne_bytes(buf[9..13].try_into().unwrap()),
                group: u32::from_ne_bytes(buf[13..17].try_into().unwrap()),
            },
            _ => panic!("protocol violation"),
        }
    }
}
