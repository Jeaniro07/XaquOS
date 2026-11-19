//! The commands a HID device on an I2C bus takes.
//!
//! A command is four bytes: the command register, the report type, and the
//! opcode. A command that carries data is written after those four bytes,
//! in the same transfer; a command that only asks is written alone, and its
//! answer is read in a second transfer.
//!
//! The opcodes are the ones Linux's `drivers/hid/i2c-hid.c` names: reset
//! (0x01), get report (0x02), set report (0x03), get idle (0x04), set idle
//! (0x05), get protocol (0x06), set protocol (0x07), and set power (0x08).
//! The three the specification defines but Linux does not use -- idle and
//! protocol -- are here as well, so that a driver can ask.
//!
//! A report command names the report in its data, not in its header: the
//! header's third byte is the report *type* -- input, output or feature --
//! and the data begins with the report's identifier and the data register
//! the descriptor named, as Linux writes them.

/// The report a command is about.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReportType {
    /// An input report: from the device.
    Input,
    /// An output report: to the device.
    Output,
    /// A feature report: both ways.
    Feature,
}

impl ReportType {
    /// The byte this report type is written as.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            ReportType::Input => 1,
            ReportType::Output => 2,
            ReportType::Feature => 3,
        }
    }
}

/// What a command asks for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Opcode {
    /// Reset the device: 0x01.
    Reset,
    /// Ask for a report: 0x02.
    GetReport,
    /// Give a report: 0x03.
    SetReport,
    /// Ask how long the device may stay idle: 0x04.
    GetIdle,
    /// Say how long the device may stay idle: 0x05.
    SetIdle,
    /// Ask which protocol the device speaks: 0x06.
    GetProtocol,
    /// Say which protocol the device speaks: 0x07.
    SetProtocol,
    /// Say whether the device is to be powered: 0x08.
    SetPower,
}

impl Opcode {
    /// The byte this opcode is written as.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Opcode::Reset => 0x01,
            Opcode::GetReport => 0x02,
            Opcode::SetReport => 0x03,
            Opcode::GetIdle => 0x04,
            Opcode::SetIdle => 0x05,
            Opcode::GetProtocol => 0x06,
            Opcode::SetProtocol => 0x07,
            Opcode::SetPower => 0x08,
        }
    }

    /// The opcode a byte names.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Opcode> {
        Some(match code {
            0x01 => Opcode::Reset,
            0x02 => Opcode::GetReport,
            0x03 => Opcode::SetReport,
            0x04 => Opcode::GetIdle,
            0x05 => Opcode::SetIdle,
            0x06 => Opcode::GetProtocol,
            0x07 => Opcode::SetProtocol,
            0x08 => Opcode::SetPower,
            _ => return None,
        })
    }
}

/// A command, and the data it carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command<'a> {
    /// Reset the device. No data.
    Reset,
    /// Ask for a report of `kind` with `id`.
    GetReport {
        /// The kind of report.
        kind: ReportType,
        /// The report's identifier.
        id: u8,
    },
    /// Give a report of `kind` with `id`, and its bytes.
    SetReport {
        /// The kind of report.
        kind: ReportType,
        /// The report's identifier.
        id: u8,
        /// The report's bytes.
        data: &'a [u8],
    },
    /// Ask how long the device may stay idle.
    GetIdle,
    /// Say the device may stay idle for `milliseconds`.
    SetIdle {
        /// The idle time, in milliseconds.
        milliseconds: u16,
    },
    /// Ask which protocol the device speaks.
    GetProtocol,
    /// Say the device speaks `protocol`: 0 for the boot protocol, 1 for the
    /// report protocol.
    SetProtocol {
        /// The protocol.
        protocol: u8,
    },
    /// Say whether the device is to be powered: 0 for on, 1 for sleep.
    SetPower {
        /// The state.
        state: u8,
    },
}

impl<'a> Command<'a> {
    /// The opcode this command is.
    #[must_use]
    pub const fn opcode(&self) -> Opcode {
        match self {
            Command::Reset => Opcode::Reset,
            Command::GetReport { .. } => Opcode::GetReport,
            Command::SetReport { .. } => Opcode::SetReport,
            Command::GetIdle => Opcode::GetIdle,
            Command::SetIdle { .. } => Opcode::SetIdle,
            Command::GetProtocol => Opcode::GetProtocol,
            Command::SetProtocol { .. } => Opcode::SetProtocol,
            Command::SetPower { .. } => Opcode::SetPower,
        }
    }

    /// The report type this command carries, as it is written in the
    /// header: none for a command that is about no report.
    #[must_use]
    pub const fn kind(&self) -> Option<ReportType> {
        match self {
            Command::GetReport { kind, .. } | Command::SetReport { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// How many bytes this command is written as: its four header bytes,
    /// then its data, if any.
    #[must_use]
    pub fn length(&self) -> usize {
        match self {
            Command::GetReport { .. } => 7,
            Command::SetReport { data, .. } => 7 + data.len(),
            Command::SetIdle { .. } => 6,
            Command::SetProtocol { .. } | Command::SetPower { .. } => 5,
            Command::Reset | Command::GetIdle | Command::GetProtocol => 4,
        }
    }

    /// Write this command at the start of `out`, as it is written to the
    /// command register, and answer how many bytes it took.
    ///
    /// `command_register` is the command register the descriptor named, and
    /// `data_register` the data register it named; a report command writes
    /// the latter in its data, as Linux does. The buffer must be at least
    /// [`Command::length`] bytes long; a shorter one takes what fits.
    #[must_use]
    pub fn encode(&self, command_register: u16, data_register: u16, out: &mut [u8]) -> usize {
        let header = [
            (command_register & 0xFF) as u8,
            ((command_register >> 8) & 0xFF) as u8,
            self.kind().map_or(0, ReportType::code),
            self.opcode().code(),
        ];
        let mut at = 0;
        for byte in header {
            match out.get_mut(at) {
                Some(slot) => *slot = byte,
                None => break,
            }
            at += 1;
        }
        match self {
            Command::GetReport { id, .. } => {
                let head = report_head(*id, data_register);
                put(out, at, &head)
            }
            Command::SetReport {
                id, data: payload, ..
            } => {
                let head = report_head(*id, data_register);
                let at = put(out, at, &head);
                put(out, at, payload)
            }
            Command::SetIdle { milliseconds } => {
                let bytes = milliseconds.to_le_bytes();
                put(out, at, &bytes)
            }
            Command::SetProtocol { protocol } => put(out, at, core::slice::from_ref(protocol)),
            Command::SetPower { state } => put(out, at, core::slice::from_ref(state)),
            Command::Reset | Command::GetIdle | Command::GetProtocol => at,
        }
    }
}

/// The three bytes a report command's data begins with: the report's
/// identifier and the data register, as Linux writes them.
fn report_head(id: u8, data_register: u16) -> [u8; 3] {
    let mut head = [0_u8; 3];
    if let Some(slot) = head.get_mut(0) {
        *slot = id;
    }
    if let Some(slot) = head.get_mut(1) {
        *slot = (data_register & 0xFF) as u8;
    }
    if let Some(slot) = head.get_mut(2) {
        *slot = ((data_register >> 8) & 0xFF) as u8;
    }
    head
}

/// Write `bytes` at `at` in `out`, and answer where the next byte goes.
fn put(out: &mut [u8], at: usize, bytes: &[u8]) -> usize {
    let mut at = at;
    for byte in bytes {
        match out.get_mut(at) {
            Some(slot) => *slot = *byte,
            None => break,
        }
        at += 1;
    }
    at
}
