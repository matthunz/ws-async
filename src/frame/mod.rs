pub use ws_frame::Opcode;

mod payload;
pub use payload::Payload;

#[derive(Debug)]
pub struct Frame<P = Payload> {
    op: Opcode,
    rsv: [bool; 3],
    payload: P,
}

impl<P> Frame<P> {
    pub const fn new(op: Opcode, rsv: [bool; 3], payload: P) -> Self {
        Self { op, rsv, payload }
    }

    pub const fn binary(payload: P) -> Self {
        Self::new_default(Opcode::Binary, payload)
    }

    const fn new_default(op: Opcode, payload: P) -> Self {
        Self::new(op, [false; 3], payload)
    }

    #[inline]
    pub fn opcode(&self) -> &Opcode {
        &self.op
    }

    #[inline]
    pub fn rsv(&self) -> &[bool; 3] {
        &self.rsv
    }

    #[inline]
    pub fn payload(&self) -> &P {
        &self.payload
    }

    #[inline]
    pub fn into_payload(self) -> P {
        self.payload
    }
}
