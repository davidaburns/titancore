#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LogonOpcode {
    CmdAuthLogonChallenge = 0x00,
    CmdAuthLogonProof = 0x01,
    CmdAuthReconnectChallenge = 0x02,
    CmdAuthReconnectProof = 0x03,
    CmdSurveyResult = 0x04,
    CmdRealmList = 0x10,
    CmdXferInitiate = 0x30,
    CmdXferData = 0x31,
    CmdXferAccept = 0x32,
    CmdXferResume = 0x33,
    CmdXferCancel = 0x34,
    CmdUknownOpcode = 0xFF,
}

impl From<u8> for LogonOpcode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => LogonOpcode::CmdAuthLogonChallenge,
            0x01 => LogonOpcode::CmdAuthLogonProof,
            0x02 => LogonOpcode::CmdAuthReconnectChallenge,
            0x03 => LogonOpcode::CmdAuthReconnectProof,
            0x04 => LogonOpcode::CmdSurveyResult,
            0x10 => LogonOpcode::CmdRealmList,
            0x30 => LogonOpcode::CmdXferInitiate,
            0x31 => LogonOpcode::CmdXferData,
            0x32 => LogonOpcode::CmdXferAccept,
            0x33 => LogonOpcode::CmdXferResume,
            0x34 => LogonOpcode::CmdXferCancel,
            _ => LogonOpcode::CmdUknownOpcode,
        }
    }
}
