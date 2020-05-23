use std::fmt;
pub use ws_frame::Opcode;

mod payload;
pub use payload::Payload;

pub struct Frame<P> {
    pub opcode: Opcode,
    pub rsv: [bool; 3],
    pub payload: P,
}

impl<P> Frame<P> {
    pub const fn new(opcode: Opcode, rsv: [bool; 3], payload: P) -> Self {
        Self {
            opcode,
            rsv,
            payload,
        }
    }

    pub const fn binary(payload: P) -> Self {
        Self::new_default(Opcode::Binary, payload)
    }

    pub const fn text(payload: P) -> Self {
        Self::new_default(Opcode::Text, payload)
    }

    const fn new_default(op: Opcode, payload: P) -> Self {
        Self::new(op, [false; 3], payload)
    }
}

impl<P> fmt::Debug for Frame<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame")
            .field("opcode", &self.opcode)
            .field("rsv", &self.rsv)
            .finish()
    }
}

#[derive(Debug)]
pub struct Raw<P> {
    pub frame: Frame<P>,
    pub mask: Option<u32>,
    pub finished: bool,
}
