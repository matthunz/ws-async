pub use ws_frame::Opcode;

mod payload;
pub use payload::Payload;

#[derive(Debug)]
pub struct Frame<P> {
    pub op: Opcode,
    pub rsv: [bool; 3],
    pub payload: P,
}

impl<P> Frame<P> {
    pub const fn new(op: Opcode, rsv: [bool; 3], payload: P) -> Self {
        Self { op, rsv, payload }
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

#[derive(Debug)]
pub struct Raw<P> {
    pub frame: Frame<P>,
    pub mask: Option<u32>,
    pub finished: bool,
}
