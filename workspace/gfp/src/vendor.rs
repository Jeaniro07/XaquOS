//! The vendor's own commands: the extension point, and empty on purpose.
//!
//! # The knowledge gap
//!
//! A fingerprint reader does more than deliver images. It has to be told
//! to capture, to enroll a finger, to match one, to store a template and
//! to load it back. Those are *vendor* requests, and the vendor's
//! protocol for them is **not published**: there is no specification, no
//! header file and no document to read the opcodes from. The values this
//! crate would need are the opcode of each request and the shape of the
//! bytes that go with it, and none of them is known.
//!
//! So the table here starts empty and stays empty until someone fills it
//! in from a capture of the real device -- a USB dump of the traffic
//! between the host and the sensor while the sensor is asked for a
//! presence check, a capture, an enrollment and a match.
//!
//! # Why this is written as code and not as a note
//!
//! Because a note is not a check. A table with no entries in it makes the
//! gap *testable*: [`VendorTable::request`] refuses every command with
//! [`Error::UnknownOpcode`], and [`Builder::build`] refuses to make a
//! table at all until every command has an opcode. A driver built on this
//! crate therefore cannot pretend to enroll a finger: it is stopped by the
//! type, at run time, with a named error.
//!
//! What is deliberately *not* here: a guessed opcode, a "typical" table
//! copied from another Goodix product, or an encoder that lays the bytes
//! out in a shape nobody has observed. A request carries its opcode and
//! its payload, and how they go on the wire is part of the same unknown.

use alloc::vec::Vec;

/// A request of the vendor's protocol.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VendorCommand {
    /// Ask whether a finger is on the sensor.
    FingerPresence,
    /// Start a capture.
    CaptureStart,
    /// Stop a capture.
    CaptureStop,
    /// Enroll the finger that was captured.
    Enroll,
    /// Match the finger that was captured.
    Match,
    /// Store an enrolled template.
    StoreTemplate,
    /// Load an enrolled template.
    LoadTemplate,
}

impl VendorCommand {
    /// Every command the protocol is expected to have.
    pub const ALL: [VendorCommand; 7] = [
        VendorCommand::FingerPresence,
        VendorCommand::CaptureStart,
        VendorCommand::CaptureStop,
        VendorCommand::Enroll,
        VendorCommand::Match,
        VendorCommand::StoreTemplate,
        VendorCommand::LoadTemplate,
    ];

    /// A name for the command, for a log line.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            VendorCommand::FingerPresence => "finger-presence",
            VendorCommand::CaptureStart => "capture-start",
            VendorCommand::CaptureStop => "capture-stop",
            VendorCommand::Enroll => "enroll",
            VendorCommand::Match => "match",
            VendorCommand::StoreTemplate => "store-template",
            VendorCommand::LoadTemplate => "load-template",
        }
    }
}

/// The opcode of one command.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Opcode {
    /// Which command this is the opcode of.
    pub command: VendorCommand,
    /// The opcode's value, as the device expects it.
    pub value: u8,
}

/// A request, ready to be sent once the wire shape is known.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VendorRequest {
    /// Which command this is.
    pub command: VendorCommand,
    /// The opcode the device expects.
    pub opcode: u8,
    /// The bytes that go with the command.
    pub payload: Vec<u8>,
}

/// Why a vendor request could not be made.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    /// The opcode of this command is not known: the vendor's protocol is
    /// not published.
    UnknownOpcode(VendorCommand),
}

/// The table of the vendor's opcodes.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct VendorTable {
    entries: Vec<Opcode>,
}

impl VendorTable {
    /// The empty table: no opcode is known.
    #[must_use]
    pub fn empty() -> VendorTable {
        VendorTable {
            entries: Vec::new(),
        }
    }

    /// A builder for a table.
    #[must_use]
    pub fn builder() -> Builder {
        Builder {
            entries: Vec::new(),
        }
    }

    /// Whether the table holds no opcode at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many opcodes the table holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The opcode of a command, when the table has one.
    #[must_use]
    pub fn opcode(&self, command: VendorCommand) -> Option<u8> {
        self.entries
            .iter()
            .find(|entry| entry.command == command)
            .map(|entry| entry.value)
    }

    /// Make the request for a command.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownOpcode`] when the table has no opcode for the
    /// command, which is the case for every command until the vendor's
    /// protocol is known.
    pub fn request(&self, command: VendorCommand, payload: &[u8]) -> Result<VendorRequest, Error> {
        match self.opcode(command) {
            Some(opcode) => Ok(VendorRequest {
                command,
                opcode,
                payload: payload.to_vec(),
            }),
            None => Err(Error::UnknownOpcode(command)),
        }
    }
}

/// The builder of a [`VendorTable`].
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Builder {
    entries: Vec<Opcode>,
}

impl Builder {
    /// Give a command its opcode.
    #[must_use]
    pub fn opcode(mut self, command: VendorCommand, value: u8) -> Builder {
        self.entries.push(Opcode { command, value });
        self
    }

    /// Make the table.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownOpcode`] for the first command that has no opcode
    /// yet. A table that is missing one is a table that would refuse that
    /// command at run time anyway, and saying so here says it at build of
    /// the driver instead.
    pub fn build(self) -> Result<VendorTable, Error> {
        for command in VendorCommand::ALL {
            if !self.entries.iter().any(|entry| entry.command == command) {
                return Err(Error::UnknownOpcode(command));
            }
        }
        Ok(VendorTable {
            entries: self.entries,
        })
    }
}
