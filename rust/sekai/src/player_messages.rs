use crate::PlayerState;

#[derive(Debug, Clone, Default)]
pub struct MsgPlayerListReq;

#[derive(Debug, Clone, PartialEq)]
pub struct MsgPlayerList {
    pub plyrs: Vec<PlayerState>,
}
