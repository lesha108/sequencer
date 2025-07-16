#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TxRxMode {
    All144,
    All430,
    Tx144Rx430,
    Tx430Rx144,
    WrongFreq,
}

impl TryInto<u32> for TxRxMode {
    type Error = ();

    fn try_into(self) -> Result<u32, Self::Error> {
        match self {
            TxRxMode::WrongFreq => Ok(0),
            TxRxMode::All144 => Ok(0b100),
            TxRxMode::All430 => Ok(0b101),
            TxRxMode::Tx144Rx430 => Ok(0b110),
            TxRxMode::Tx430Rx144 => Ok(0b111),
        }
    }
}

impl TryFrom<u32> for TxRxMode {
    type Error = ();

    fn try_from(a: u32) -> Result<Self, Self::Error> {
        match a {
            0 => Ok(TxRxMode::WrongFreq),
            0b100 => Ok(TxRxMode::All144),
            0b101 => Ok(TxRxMode::All430),
            0b110 => Ok(TxRxMode::Tx144Rx430),
            0b111 => Ok(TxRxMode::Tx430Rx144),
            _ => Ok(TxRxMode::WrongFreq),
        }
    }
}


pub fn code_band(rxf: u64, txf: u64) -> u32 {
    let mut mode = TxRxMode::WrongFreq;
    match rxf {
        143_000_000..146_000_000 => {
            match txf {
                143_000_000..146_000_000 => {
                    mode = TxRxMode::All144;
                },
                429_000_000..450_000_000 => {
                    mode = TxRxMode::Tx430Rx144;
                },
                _ => {}
            }
        },
        429_000_000..450_000_000 => {
            match txf {
                143_000_000..146_000_000 => {
                    mode = TxRxMode::Tx144Rx430;
                },
                429_000_000..450_000_000 => {
                    mode = TxRxMode::All430;
                },
                _ => {}
            }            
        },
        _ => {},
    }
    mode.try_into().unwrap()
}