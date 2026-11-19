//! The host command and response framing of the PCIe transport.
//!
//! A host command is what the driver sends to the firmware: a header and a
//! payload. The header is either the short one (`struct iwl_cmd_header`,
//! four bytes) or the wide one (`struct iwl_cmd_header_wide`, eight bytes),
//! both read from
//! `drivers/net/wireless/intel/iwlwifi/fw/api/cmdhdr.h` (Linux v6.6).
//!
//! * The short header is `cmd` (one byte), `group_id` (one byte) and
//!   `sequence` (a little-endian `u16`).
//! * The wide header is `cmd`, `group_id`, `sequence`, `length` (a
//!   little-endian `u16` that counts the payload only), `reserved` and
//!   `version`.
//!
//! The `cmd` byte is the opcode and the `group_id` byte is the command
//! group; together with the version they make the 32-bit command id that
//! `iwl_cmd_id`, `iwl_cmd_opcode`, `iwl_cmd_groupid` and `iwl_cmd_version`
//! in the same header read and write.
//!
//! The response to a command carries a status word: `struct
//! iwl_cmd_response` is a single little-endian `u32`. The alive
//! notification is the exception — it carries a 16-bit status, of which
//! `IWL_ALIVE_STATUS_OK` (0xCAFE) and `IWL_ALIVE_STATUS_ERR` (0xDEAD) are
//! named in `drivers/net/wireless/intel/iwlwifi/fw/api/alive.h`.
//!
//! The sequence number packs three things, as `fw/api/cmdhdr.h` says:
//!
//! ```text
//!  0:7    the transmit descriptor index
//!  8:12   the transmit queue id
//!  13:14  reserved
//!  15     set by the firmware for a notification it originated
//! ```

use core::fmt;

/// The length of the short command header.
pub const SHORT_HEADER_LEN: usize = 4;

/// The length of the wide command header.
pub const WIDE_HEADER_LEN: usize = 8;

/// The bit the firmware sets in a sequence number for a notification it
/// originated rather than answered.
///
/// Upstream: `SEQ_RX_FRAME`, `fw/api/cmdhdr.h`.
pub const SEQ_RX_FRAME: u16 = 0x8000;

/// The command groups of the firmware.
///
/// Upstream: `enum iwl_mvm_command_groups`, `fw/api/commands.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CommandGroup {
    /// `LEGACY_GROUP` (0x0), the commands of `enum iwl_legacy_cmds`.
    Legacy = 0x0,
    /// `LONG_GROUP` (0x1), the same commands with the wide header.
    Long = 0x1,
    /// `SYSTEM_GROUP` (0x2).
    System = 0x2,
    /// `MAC_CONF_GROUP` (0x3).
    MacConf = 0x3,
    /// `PHY_OPS_GROUP` (0x4).
    PhyOps = 0x4,
    /// `DATA_PATH_GROUP` (0x5).
    DataPath = 0x5,
    /// `SCAN_GROUP` (0x6).
    Scan = 0x6,
    /// `NAN_GROUP` (0x7).
    Nan = 0x7,
    /// `LOCATION_GROUP` (0x8).
    Location = 0x8,
    /// `PROT_OFFLOAD_GROUP` (0xb).
    ProtOffload = 0xb,
    /// `REGULATORY_AND_NVM_GROUP` (0xc).
    RegulatoryAndNvm = 0xc,
    /// `DEBUG_GROUP` (0xf).
    Debug = 0xf,
}

impl CommandGroup {
    /// The group with the given raw value, if the firmware names it.
    pub fn from_raw(raw: u8) -> Option<CommandGroup> {
        match raw {
            0x0 => Some(CommandGroup::Legacy),
            0x1 => Some(CommandGroup::Long),
            0x2 => Some(CommandGroup::System),
            0x3 => Some(CommandGroup::MacConf),
            0x4 => Some(CommandGroup::PhyOps),
            0x5 => Some(CommandGroup::DataPath),
            0x6 => Some(CommandGroup::Scan),
            0x7 => Some(CommandGroup::Nan),
            0x8 => Some(CommandGroup::Location),
            0xb => Some(CommandGroup::ProtOffload),
            0xc => Some(CommandGroup::RegulatoryAndNvm),
            0xf => Some(CommandGroup::Debug),
            _ => None,
        }
    }

    /// The raw value of the group.
    pub fn raw(self) -> u8 {
        self as u8
    }
}

/// The opcodes of the legacy group that a bring-up needs.
///
/// Upstream: `enum iwl_legacy_cmds`, `fw/api/commands.h`.
pub mod legacy {
    /// `UCODE_ALIVE_NTFY` (0x01): the alive notification.
    pub const UCODE_ALIVE_NTFY: u8 = 0x01;
    /// `INIT_COMPLETE_NOTIF` (0x04): initialisation is complete.
    pub const INIT_COMPLETE_NOTIF: u8 = 0x04;
    /// `PHY_CONFIGURATION_CMD` (0x6a): the PHY configuration command.
    pub const PHY_CONFIGURATION_CMD: u8 = 0x6a;
    /// `NVM_ACCESS_CMD` (0x88): the NVM access command.
    pub const NVM_ACCESS_CMD: u8 = 0x88;
    /// `TX_ANT_CONFIGURATION_CMD` (0x98): the transmit antenna command.
    pub const TX_ANT_CONFIGURATION_CMD: u8 = 0x98;
    /// `REPLY_RX_PHY_CMD` (0xc0): the PHY information of a received frame.
    pub const REPLY_RX_PHY_CMD: u8 = 0xc0;
    /// `REPLY_RX_MPDU_CMD` (0xc1): a received MPDU.
    pub const REPLY_RX_MPDU_CMD: u8 = 0xc1;
}

/// The alive status that means the firmware came up.
///
/// Upstream: `IWL_ALIVE_STATUS_OK`, `fw/api/alive.h`.
pub const ALIVE_STATUS_OK: u16 = 0xcafe;

/// The alive status that means the firmware did not come up.
///
/// Upstream: `IWL_ALIVE_STATUS_ERR`, `fw/api/alive.h`.
pub const ALIVE_STATUS_ERR: u16 = 0xdead;

/// Why a command could not be framed, or a response could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The buffer is shorter than the header needs.
    Truncated {
        /// The number of bytes the header needs.
        needed: usize,
        /// The number of bytes the buffer holds.
        found: usize,
    },
    /// The payload does not fit in the buffer given.
    PayloadTooLong {
        /// The length of the payload.
        len: usize,
        /// The size of the buffer.
        capacity: usize,
    },
    /// A header's length field disagrees with the buffer.
    LengthMismatch {
        /// The length the header declares.
        declared: usize,
        /// The length of the buffer.
        found: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated { needed, found } => {
                write!(f, "header needs {needed} bytes, got {found}")
            }
            Error::PayloadTooLong { len, capacity } => {
                write!(f, "payload of {len} bytes does not fit in {capacity}")
            }
            Error::LengthMismatch { declared, found } => {
                write!(f, "header declares {declared} bytes, got {found}")
            }
        }
    }
}

impl core::error::Error for Error {}

/// A command id: the opcode, the group and the version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandId {
    opcode: u8,
    group: u8,
    version: u8,
}

impl CommandId {
    /// A command id from its parts.
    pub fn new(opcode: u8, group: CommandGroup, version: u8) -> CommandId {
        CommandId {
            opcode,
            group: group.raw(),
            version,
        }
    }

    /// A command id from the 32-bit form the kernel uses.
    pub fn from_raw(raw: u32) -> CommandId {
        CommandId {
            opcode: (raw & 0xff) as u8,
            group: ((raw & 0xff00) >> 8) as u8,
            version: ((raw & 0xff_0000) >> 16) as u8,
        }
    }

    /// The 32-bit form the kernel uses.
    pub fn raw(self) -> u32 {
        (self.opcode as u32) | ((self.group as u32) << 8) | ((self.version as u32) << 16)
    }

    /// The opcode of the command.
    pub fn opcode(self) -> u8 {
        self.opcode
    }

    /// The raw group byte of the command.
    pub fn group(self) -> u8 {
        self.group
    }

    /// The group of the command, if the firmware names it.
    pub fn group_kind(self) -> Option<CommandGroup> {
        CommandGroup::from_raw(self.group)
    }

    /// The version of the command.
    pub fn version(self) -> u8 {
        self.version
    }
}

/// The short command header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CmdHeader {
    cmd: u8,
    group_id: u8,
    sequence: u16,
}

impl CmdHeader {
    /// The length of the header on the wire.
    pub const LEN: usize = SHORT_HEADER_LEN;

    /// A header from its parts.
    pub fn new(cmd: u8, group_id: u8, sequence: u16) -> CmdHeader {
        CmdHeader {
            cmd,
            group_id,
            sequence,
        }
    }

    /// Reads a short header out of the start of a buffer.
    pub fn parse(bytes: &[u8]) -> Result<CmdHeader, Error> {
        if bytes.len() < SHORT_HEADER_LEN {
            return Err(Error::Truncated {
                needed: SHORT_HEADER_LEN,
                found: bytes.len(),
            });
        }
        Ok(CmdHeader {
            cmd: read_u8(bytes, 0),
            group_id: read_u8(bytes, 1),
            sequence: read_u16(bytes, 2),
        })
    }

    /// The header as bytes.
    pub fn encode(&self) -> [u8; SHORT_HEADER_LEN] {
        let mut out = [0_u8; SHORT_HEADER_LEN];
        if let Some(slot) = out.get_mut(0) {
            *slot = self.cmd;
        }
        if let Some(slot) = out.get_mut(1) {
            *slot = self.group_id;
        }
        if let Some(slot) = out.get_mut(2..4) {
            slot.copy_from_slice(&self.sequence.to_le_bytes());
        }
        out
    }

    /// The opcode of the command.
    pub fn cmd(&self) -> u8 {
        self.cmd
    }

    /// The group byte of the command.
    pub fn group_id(&self) -> u8 {
        self.group_id
    }

    /// The sequence number of the command.
    pub fn sequence(&self) -> u16 {
        self.sequence
    }
}

/// The wide command header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WideCmdHeader {
    cmd: u8,
    group_id: u8,
    sequence: u16,
    length: u16,
    reserved: u8,
    version: u8,
}

impl WideCmdHeader {
    /// The length of the header on the wire.
    pub const LEN: usize = WIDE_HEADER_LEN;

    /// A header from its parts.
    pub fn new(
        cmd: u8,
        group_id: u8,
        sequence: u16,
        length: u16,
        reserved: u8,
        version: u8,
    ) -> WideCmdHeader {
        WideCmdHeader {
            cmd,
            group_id,
            sequence,
            length,
            reserved,
            version,
        }
    }

    /// Reads a wide header out of the start of a buffer.
    pub fn parse(bytes: &[u8]) -> Result<WideCmdHeader, Error> {
        if bytes.len() < WIDE_HEADER_LEN {
            return Err(Error::Truncated {
                needed: WIDE_HEADER_LEN,
                found: bytes.len(),
            });
        }
        Ok(WideCmdHeader {
            cmd: read_u8(bytes, 0),
            group_id: read_u8(bytes, 1),
            sequence: read_u16(bytes, 2),
            length: read_u16(bytes, 4),
            reserved: read_u8(bytes, 6),
            version: read_u8(bytes, 7),
        })
    }

    /// The header as bytes.
    pub fn encode(&self) -> [u8; WIDE_HEADER_LEN] {
        let mut out = [0_u8; WIDE_HEADER_LEN];
        if let Some(slot) = out.get_mut(0) {
            *slot = self.cmd;
        }
        if let Some(slot) = out.get_mut(1) {
            *slot = self.group_id;
        }
        if let Some(slot) = out.get_mut(2..4) {
            slot.copy_from_slice(&self.sequence.to_le_bytes());
        }
        if let Some(slot) = out.get_mut(4..6) {
            slot.copy_from_slice(&self.length.to_le_bytes());
        }
        if let Some(slot) = out.get_mut(6) {
            *slot = self.reserved;
        }
        if let Some(slot) = out.get_mut(7) {
            *slot = self.version;
        }
        out
    }

    /// The opcode of the command.
    pub fn cmd(&self) -> u8 {
        self.cmd
    }

    /// The group byte of the command.
    pub fn group_id(&self) -> u8 {
        self.group_id
    }

    /// The sequence number of the command.
    pub fn sequence(&self) -> u16 {
        self.sequence
    }

    /// The length of the payload, in bytes.
    pub fn length(&self) -> u16 {
        self.length
    }

    /// The reserved byte.
    pub fn reserved(&self) -> u8 {
        self.reserved
    }

    /// The version of the command.
    pub fn version(&self) -> u8 {
        self.version
    }
}

/// The transmit queue id in a sequence number.
///
/// Upstream: `SEQ_TO_QUEUE`, `fw/api/cmdhdr.h`.
pub fn seq_to_queue(sequence: u16) -> u8 {
    ((sequence >> 8) & 0x1f) as u8
}

/// The sequence number that names a transmit queue.
///
/// Upstream: `QUEUE_TO_SEQ`, `fw/api/cmdhdr.h`.
pub fn queue_to_seq(queue: u8) -> u16 {
    (queue as u16 & 0x1f) << 8
}

/// The transmit descriptor index in a sequence number.
///
/// Upstream: `SEQ_TO_INDEX`, `fw/api/cmdhdr.h`.
pub fn seq_to_index(sequence: u16) -> u8 {
    (sequence & 0xff) as u8
}

/// The status word of a response.
///
/// Upstream: `struct iwl_cmd_response`, `fw/api/cmdhdr.h`.
pub fn response_status(bytes: &[u8]) -> Result<u32, Error> {
    if bytes.len() < 4 {
        return Err(Error::Truncated {
            needed: 4,
            found: bytes.len(),
        });
    }
    Ok(read_u32(bytes, 0))
}

/// Frames a command: the wide header, then the payload.
///
/// The `length` in the header is the length of the payload, as the kernel's
/// own framing is. The number of bytes written is returned.
pub fn frame_command(
    id: CommandId,
    sequence: u16,
    payload: &[u8],
    out: &mut [u8],
) -> Result<usize, Error> {
    if payload.len() > u16::MAX as usize {
        return Err(Error::PayloadTooLong {
            len: payload.len(),
            capacity: u16::MAX as usize,
        });
    }
    let needed = WIDE_HEADER_LEN + payload.len();
    if out.len() < needed {
        return Err(Error::PayloadTooLong {
            len: needed,
            capacity: out.len(),
        });
    }
    let header = WideCmdHeader::new(
        id.opcode(),
        id.group(),
        sequence,
        payload.len() as u16,
        0,
        id.version(),
    );
    let encoded = header.encode();
    if let Some(slot) = out.get_mut(..WIDE_HEADER_LEN) {
        slot.copy_from_slice(&encoded);
    }
    if let Some(slot) = out.get_mut(WIDE_HEADER_LEN..needed) {
        slot.copy_from_slice(payload);
    }
    Ok(needed)
}

/// Reads a response: its wide header and the length it declares.
///
/// The returned header and the payload's length are checked against each
/// other, so a response whose length field does not match the buffer is an
/// error rather than a shorter read.
pub fn parse_response(bytes: &[u8]) -> Result<(WideCmdHeader, &[u8]), Error> {
    let header = WideCmdHeader::parse(bytes)?;
    let payload = bytes.get(WIDE_HEADER_LEN..).unwrap_or(&[]);
    if payload.len() != header.length() as usize {
        return Err(Error::LengthMismatch {
            declared: header.length() as usize,
            found: payload.len(),
        });
    }
    Ok((header, payload))
}

/// Reads the little-endian `u8` at `at`, or zero if the byte is not there.
fn read_u8(bytes: &[u8], at: usize) -> u8 {
    bytes.get(at).copied().unwrap_or(0)
}

/// Reads the little-endian `u16` at `at`, or zero if the bytes are not there.
fn read_u16(bytes: &[u8], at: usize) -> u16 {
    bytes
        .get(at..at.saturating_add(2))
        .and_then(|field| <[u8; 2]>::try_from(field).ok())
        .map(u16::from_le_bytes)
        .unwrap_or(0)
}

/// Reads the little-endian `u32` at `at`, or zero if the bytes are not there.
fn read_u32(bytes: &[u8], at: usize) -> u32 {
    bytes
        .get(at..at.saturating_add(4))
        .and_then(|field| <[u8; 4]>::try_from(field).ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0)
}
