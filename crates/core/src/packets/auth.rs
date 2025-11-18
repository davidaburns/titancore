pub enum AuthPacketOpcode {
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
}
