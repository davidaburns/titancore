use crate::opcode::LogonOpcode;
use anyhow::{Result, anyhow};
use tc_core::server::Packet;

pub struct LogonPacket {
    pub opcode: LogonOpcode,
    pub payload: Vec<u8>,
}

impl Packet for LogonPacket {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn decode(payload: &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        if payload.len() == 0 {
            return Err(anyhow!("packet payload is empty"));
        }

        let op = payload[0];
        let payload = if payload.len() >= 2 {
            payload[1..]
                .to_vec()
                .iter()
                .copied()
                .filter(|&b| b != 0x0D && b != 0x0A)
                .collect()
        } else {
            Vec::new()
        };

        Ok(Self {
            opcode: LogonOpcode::from(op),
            payload,
        })
    }
}

const AUTH_LOGON_CHALLENGE_REQUEST_LEN: usize = 1 + 2 + 4 + 1 + 1 + 1 + 2 + 4 + 4 + 4 + 4 + 4 + 1;

#[derive(Debug)]
pub struct AuthLogonChallengeRequest {
    pub opcode: u8,
    pub error: u8,
    pub size: u16,
    pub game_name: [u8; 4],
    pub version1: u8,
    pub version2: u8,
    pub version3: u8,
    pub build: u16,
    pub platform: [u8; 4],
    pub os: [u8; 4],
    pub country: [u8; 4],
    pub timezone_bias: u32,
    pub ip: u32,
    pub account_name_len: u8,
    pub account_name: Vec<u8>,
}

impl TryFrom<LogonPacket> for AuthLogonChallengeRequest {
    type Error = anyhow::Error;
    fn try_from(value: LogonPacket) -> std::result::Result<Self, Self::Error> {
        if value.payload.len() <= AUTH_LOGON_CHALLENGE_REQUEST_LEN {
            return Err(anyhow!(
                "Payload for AuthLogonChallengeRequest is not of length: {AUTH_LOGON_CHALLENGE_REQUEST_LEN}"
            ));
        }

        Ok(Self {
            opcode: value.opcode as u8,
            error: value.payload[0],
            size: u16::from_le_bytes([value.payload[1], value.payload[2]]),
            game_name: [
                value.payload[3],
                value.payload[4],
                value.payload[5],
                value.payload[6],
            ],
            version1: value.payload[7],
            version2: value.payload[8],
            version3: value.payload[9],
            build: u16::from_le_bytes([value.payload[10], value.payload[11]]),
            platform: [
                value.payload[15],
                value.payload[14],
                value.payload[13],
                value.payload[12],
            ],
            os: [
                value.payload[19],
                value.payload[18],
                value.payload[17],
                value.payload[16],
            ],
            country: [
                value.payload[23],
                value.payload[22],
                value.payload[21],
                value.payload[20],
            ],
            timezone_bias: u32::from_le_bytes([
                value.payload[27],
                value.payload[26],
                value.payload[25],
                value.payload[24],
            ]),
            ip: u32::from_be_bytes([
                value.payload[28],
                value.payload[29],
                value.payload[30],
                value.payload[31],
            ]),
            account_name_len: value.payload[32],
            account_name: Vec::from(&value.payload[33..]),
        })
    }
}

impl Into<LogonPacket> for AuthLogonChallengeRequest {
    fn into(self) -> LogonPacket {
        todo!()
    }
}

pub struct AuthLogonChallengeResponse {
    cmd: u8,
    error: u8,
    b: [u8; 32],
    g_len: u8,
    g: u8,
    n_len: u8,
    n: [u8; 32],
    s: [u8; 32],
    unknown: [u8; 16],
    security_flags: u8,
}

impl Into<LogonPacket> for AuthLogonChallengeResponse {
    fn into(self) -> LogonPacket {
        todo!()
    }
}

pub struct AuthLogonProofRequest {
    cmd: u8,
    a: [u8; 32],
    m1: [u8; 20],
    crc_hash: [u8; 20],
    number_of_keys: u8,
    security_flags: u8,
}

impl TryFrom<LogonPacket> for AuthLogonProofRequest {
    type Error = anyhow::Error;
    fn try_from(value: LogonPacket) -> std::result::Result<Self, Self::Error> {
        todo!()
    }
}

pub struct AuthLogonProofResponse {
    cmd: u8,
    error: u8,
    m2: [u8; 20],
    account_flags: u32,
    survey_id: u32,
    login_flags: u16,
}

impl Into<LogonPacket> for AuthLogonProofResponse {
    fn into(self) -> LogonPacket {
        todo!()
    }
}

pub struct AuthReconnectChallengeRequest {
    cmd: u8,
    error: u8,
    size: u16,
    game_name: [u8; 4],
    version1: u8,
    version2: u8,
    version3: u8,
    build: u16,
    platform: [u8; 4],
    os: [u8; 4],
    country: [u8; 4],
    timezone_bias: u32,
    ip: u32,
    account_name_len: u8,
    account_name: Vec<u8>,
}

impl TryFrom<LogonPacket> for AuthReconnectChallengeRequest {
    type Error = anyhow::Error;
    fn try_from(value: LogonPacket) -> std::result::Result<Self, Self::Error> {
        todo!()
    }
}

pub struct AuthReconnectChallengeResponse {
    cmd: u8,
    error: u8,
    challenge_data: [u8; 16],
    checksum_salt: [u8; 16],
}

impl Into<LogonPacket> for AuthReconnectChallengeResponse {
    fn into(self) -> LogonPacket {
        todo!()
    }
}

pub struct AuthReconnectProofRequest {
    cmd: u8,
    r1: [u8; 16],
    r2: [u8; 20],
    r3: [u8; 20],
    number_of_keys: u8,
}

impl TryFrom<LogonPacket> for AuthReconnectProofRequest {
    type Error = anyhow::Error;
    fn try_from(value: LogonPacket) -> std::result::Result<Self, Self::Error> {
        todo!()
    }
}

pub struct AuthReconnectProofResponse {
    cmd: u8,
    error: u8,
}

impl Into<LogonPacket> for AuthReconnectProofResponse {
    fn into(self) -> LogonPacket {
        todo!()
    }
}

pub struct AuthRealmlistRequest {
    cmd: u8,
    unknown: u32,
}

impl TryFrom<LogonPacket> for AuthRealmlistRequest {
    type Error = anyhow::Error;
    fn try_from(value: LogonPacket) -> std::result::Result<Self, Self::Error> {
        todo!()
    }
}
