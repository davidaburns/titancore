use crate::server::ClientHandle;
use std::net::SocketAddr;

pub enum ServerMessage {
    AddClient((SocketAddr, ClientHandle)),
    ClientDisconnected(SocketAddr),
}

pub enum ClientMessage {
    Send(Vec<u8>),
}
