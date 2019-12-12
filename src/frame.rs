use crate::Payload;

#[derive(Debug, PartialEq)]
pub enum Opcode {
    Continue,
    Text,
    Binary,
    Close,
    Ping,
    Pong,
    Reserved(u8),
}

impl Opcode {
    pub fn to_byte(&self) -> u8 {
        use Opcode::*;

        match self {
            Continue => 1,
            Text => 2,
            Binary => 3,
            Close => 8,
            Ping => 9,
            Pong => 10,
            Reserved(op) => op,
        }
    }
}

impl From<u8> for Opcode {
    fn from(opcode: u8) -> Opcode {
        use Opcode::*;

        match opcode {
            0 => Continue,
            1 => Text,
            2 => Binary,
            8 => Close,
            9 => Ping,
            10 => Pong,
            op => Reserved(op),
        }
    }
}

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
