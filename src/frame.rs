use crate::Payload;
pub use ws_frame::Opcode;

#[derive(Debug)]
pub struct Frame<P = Payload> {
    op: Opcode,
    rsv: [bool; 3],
    payload: P,
}

impl<P> Frame<P> {
    pub fn new(op: Opcode, rsv: [bool; 3], payload: P) -> Self {
        Self { op, rsv, payload }
    }
    pub fn binary(payload: P) -> Self {
        Self::new_default(Opcode::Binary, payload)
    }
    fn new_default(op: Opcode, payload: P) -> Self {
        Self::new(op, [false; 3], payload)
    }
    pub fn opcode(&self) -> &Opcode {
        &self.op
    }
    pub fn rsv(&self) -> &[bool; 3] {
        &self.rsv
    }
    pub fn payload(&self) -> &P {
        &self.payload
    }
    pub fn into_payload(self) -> P {
        self.payload
    }
}
