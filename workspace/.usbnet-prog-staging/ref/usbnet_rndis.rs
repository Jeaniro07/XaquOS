//! The Remote NDIS protocol: how a dongle that Microsoft designed carries
//! Ethernet frames.
//!
//! RNDIS is a message protocol over the CDC class: every message is sent
//! in the data stage of a `SEND_ENCAPSULATED_COMMAND`, and the answer
//! comes back through `GET_ENCAPSULATED_RESPONSE`. A message starts with
//! a header naming its type, its length, the request it belongs to and the
//! status of the answer.
//!
//! The protocol has states, and a driver that skips one of them gets a
//! device that does not work: the device must be initialized with an
//! `INIT` before anything else, the station's address is then read with
//! an OID query, and only then may frames be sent. `drivers/net/usb/
//! rndis_host.c` is the reference for the order and for the framing.
//!
//! The message types, the header layouts and the identifiers are the
//! kernel's own, as `include/linux/usb/rndis_host.h` and
//! `drivers/net/usb/rndis_host.c` read them.

use crate::bytes::{get32, put16, put32};
use crate::cdc::{self, ClassRequest};

/// The message types, as `rndis_host.h` numbers them.
pub mod message_type {
    /// PACKET: a frame on its way in or out.
    pub const PACKET: u32 = 1;
    /// INIT: the initialization message, which comes first.
    pub const INIT: u32 = 2;
    /// HALT: the message that shuts the device down.
    pub const HALT: u32 = 3;
    /// QUERY: read an object identifier.
    pub const QUERY: u32 = 4;
    /// SET: write an object identifier.
    pub const SET: u32 = 5;
    /// RESET: reset the device.
    pub const RESET: u32 = 6;
    /// INDICATE: a message the device sends unasked.
    pub const INDICATE: u32 = 7;
    /// KEEPALIVE: the message that says the device is still there.
    pub const KEEPALIVE: u32 = 8;
    /// The bit a reply carries: it is set on every answer to a request.
    pub const COMPLETION_FLAG: u32 = 0x8000_0000;
}

/// The statuses a reply carries, as `rndis_host.h` numbers them.
pub mod status {
    /// SUCCESS: the request was carried out.
    pub const SUCCESS: u32 = 0;
    /// FAILURE: the request was refused.
    pub const FAILURE: u32 = 0xC000_0001;
    /// `INVALID_DATA`: the request's data was not understood.
    pub const INVALID_DATA: u32 = 0xC001_0015;
    /// `NOT_SUPPORTED`: the device does not have that object identifier.
    pub const NOT_SUPPORTED: u32 = 0xC000_00BB;
    /// `MEDIA_CONNECT`: the link is up.
    pub const MEDIA_CONNECT: u32 = 0x4001_000B;
}

/// The object identifiers a driver asks the device about, as
/// `rndis_host.h` numbers them.
pub mod oid {
    /// `MEDIA_CONNECT_STATUS`: whether the link is up.
    pub const GEN_MEDIA_CONNECT_STATUS: u32 = 0x0001_0114;
    /// `PERMANENT_ADDRESS`: the address the station was given.
    pub const PERMANENT_ADDRESS: u32 = 0x0101_0101;
    /// `CURRENT_ADDRESS`: the address the station is using now.
    pub const CURRENT_ADDRESS: u32 = 0x0101_0102;
}

/// The bytes a message header takes.
pub const MESSAGE_HEADER_BYTES: usize = 16;

/// The length a message header says a message has.
pub const MESSAGE_HEADER_LENGTH: u32 = 16;

/// The bytes a data message's header takes, up to the payload.
///
/// The header is eight 32-bit fields, which is thirty-six bytes, and the
/// payload starts at the thirty-sixth byte: `RNDIS_MSG_DATA_OFFSET`.
pub const DATA_HEADER_BYTES: usize = 36;

/// The offset the payload of a data message starts at, which is
/// `RNDIS_MSG_DATA_OFFSET`.
pub const DATA_OFFSET: usize = 36;

/// The bytes an initialization message takes: a message header and three
/// more fields.
pub const INIT_BYTES: usize = 28;

/// The length an initialization message says it has.
pub const INIT_LENGTH: u32 = 28;

/// The bytes a query or a set message takes.
pub const QUERY_BYTES: usize = 28;

/// The length a query or a set message says it has.
pub const QUERY_LENGTH: u32 = 28;

/// The offset the information buffer of a query or a set starts at, which
/// is `RNDIS_MSG_INFO_BUFFER_OFFSET`.
pub const INFORMATION_BUFFER_OFFSET: u16 = 20;

/// The header every RNDIS message starts with.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MessageHeader {
    /// `MessageType`: the message's type, with the completion bit set on a
    /// reply.
    pub message_type: u32,
    /// `MessageLength`: how long the whole message is.
    pub message_length: u32,
    /// `RequestId`: the request this message belongs to.
    pub request_id: u32,
    /// `Status`: what the request's answer says.
    pub status: u32,
}

impl MessageHeader {
    /// The bytes the header takes.
    pub const BYTES: usize = MESSAGE_HEADER_BYTES;

    /// The header's bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; Self::BYTES] {
        let mut out = [0_u8; Self::BYTES];
        put32(&mut out, 0, self.message_type);
        put32(&mut out, 4, self.message_length);
        put32(&mut out, 8, self.request_id);
        put32(&mut out, 12, self.status);
        out
    }

    /// The header the bytes carry.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let head = bytes.get(..Self::BYTES)?;
        Some(Self {
            message_type: get32(head, 0)?,
            message_length: get32(head, 4)?,
            request_id: get32(head, 8)?,
            status: get32(head, 12)?,
        })
    }

    /// The message's type without the completion bit.
    #[must_use]
    pub fn base_type(&self) -> u32 {
        self.message_type & !message_type::COMPLETION_FLAG
    }

    /// Whether the message is a reply to a request.
    #[must_use]
    pub fn is_completion(&self) -> bool {
        self.message_type & message_type::COMPLETION_FLAG != 0
    }

    /// Whether the message is the answer to the request `request_id`.
    ///
    /// Both have to match: the completion bit says it is an answer at all,
    /// and the request number says it is the answer to this one. A device
    /// answers several requests in flight, and the driver has to pick the
    /// right one.
    #[must_use]
    pub fn is_reply_to(&self, request_id: u32) -> bool {
        self.is_completion() && self.request_id == request_id
    }

    /// Whether the message says the request succeeded.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.status == status::SUCCESS
    }
}

/// The header a data message starts with.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DataHeader {
    /// `MessageType`: the message's type.
    pub message_type: u32,
    /// `MessageLength`: how long the whole message is.
    pub message_length: u32,
    /// `RequestId`: the request this message belongs to.
    pub request_id: u32,
    /// `Status`: what the request's answer says.
    pub status: u32,
    /// `Offset`: where the payload is, counted from the start of the
    /// message.
    pub offset: u32,
    /// `Length`: how long the payload is.
    pub length: u32,
    /// `DataLength`: how long the frame is.
    pub data_length: u32,
    /// `Reserved`: unused, and always zero.
    pub reserved: u32,
}

impl DataHeader {
    /// The bytes the header takes.
    pub const BYTES: usize = DATA_HEADER_BYTES;

    /// The header's bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; Self::BYTES] {
        let mut out = [0_u8; Self::BYTES];
        put32(&mut out, 0, self.message_type);
        put32(&mut out, 4, self.message_length);
        put32(&mut out, 8, self.request_id);
        put32(&mut out, 12, self.status);
        put32(&mut out, 16, self.offset);
        put32(&mut out, 20, self.length);
        put32(&mut out, 24, self.data_length);
        put32(&mut out, 28, self.reserved);
        out
    }

    /// The header the bytes carry.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let head = bytes.get(..Self::BYTES)?;
        Some(Self {
            message_type: get32(head, 0)?,
            message_length: get32(head, 4)?,
            request_id: get32(head, 8)?,
            status: get32(head, 12)?,
            offset: get32(head, 16)?,
            length: get32(head, 20)?,
            data_length: get32(head, 24)?,
            reserved: get32(head, 28)?,
        })
    }

    /// The payload the message carries.
    #[must_use]
    pub fn payload<'a>(&self, bytes: &'a [u8]) -> Option<&'a [u8]> {
        let start = usize::try_from(self.offset).ok()?;
        let end = start.checked_add(usize::try_from(self.length).ok()?)?;
        bytes.get(start..end)
    }
}

/// The initialization message, which is the first message a driver sends.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Init {
    /// `MajorVersion`: the protocol version the driver speaks.
    pub major_version: u32,
    /// `MinorVersion`: the protocol version's minor number.
    pub minor_version: u32,
    /// `MaxTransferSize`: the largest message the driver will send.
    pub max_transfer_size: u32,
}

impl Init {
    /// The bytes the message takes.
    pub const BYTES: usize = INIT_BYTES;

    /// An initialization message for the protocol version `version`.
    #[must_use]
    pub const fn new(major_version: u32, minor_version: u32, max_transfer_size: u32) -> Self {
        Self {
            major_version,
            minor_version,
            max_transfer_size,
        }
    }

    /// The message that belongs to the request `request_id`.
    #[must_use]
    pub fn message(&self, request_id: u32) -> [u8; Self::BYTES] {
        let mut out = [0_u8; Self::BYTES];
        put32(&mut out, 0, message_type::INIT);
        put32(&mut out, 4, INIT_LENGTH);
        put32(&mut out, 8, request_id);
        put32(&mut out, 12, status::SUCCESS);
        put32(&mut out, 16, self.major_version);
        put32(&mut out, 20, self.minor_version);
        put32(&mut out, 24, self.max_transfer_size);
        out
    }
}

/// A query or a set of an object identifier.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Query {
    /// `Oid`: the object identifier being read or written.
    pub oid: u32,
    /// `InformationBufferLength`: how long the information buffer is.
    pub information_buffer_length: u16,
    /// `InformationBufferOffset`: where the information buffer is, counted
    /// from the start of the message.
    pub information_buffer_offset: u16,
    /// `Reserved`: unused, and always zero.
    pub reserved: u32,
}

impl Query {
    /// The bytes the message takes.
    pub const BYTES: usize = QUERY_BYTES;

    /// A query of the object identifier `oid`, with no information
    /// buffer.
    #[must_use]
    pub const fn new(oid: u32) -> Self {
        Self {
            oid,
            information_buffer_length: 0,
            information_buffer_offset: INFORMATION_BUFFER_OFFSET,
            reserved: 0,
        }
    }

    /// The query message that belongs to the request `request_id`.
    #[must_use]
    pub fn message(&self, request_id: u32) -> [u8; Self::BYTES] {
        self.message_of_type(message_type::QUERY, request_id)
    }

    /// The set message that belongs to the request `request_id`.
    #[must_use]
    pub fn set_message(&self, request_id: u32) -> [u8; Self::BYTES] {
        self.message_of_type(message_type::SET, request_id)
    }

    /// The message of the type `message_type` that belongs to the request
    /// `request_id`.
    #[must_use]
    pub fn message_of_type(&self, message_type: u32, request_id: u32) -> [u8; Self::BYTES] {
        let mut out = [0_u8; Self::BYTES];
        put32(&mut out, 0, message_type);
        put32(&mut out, 4, QUERY_LENGTH);
        put32(&mut out, 8, request_id);
        put32(&mut out, 12, status::SUCCESS);
        put32(&mut out, 16, self.oid);
        put16(&mut out, 20, self.information_buffer_length);
        put16(&mut out, 22, self.information_buffer_offset);
        put32(&mut out, 24, self.reserved);
        out
    }
}

/// The message that shuts the device down.
#[must_use]
pub fn halt_message(request_id: u32) -> [u8; MESSAGE_HEADER_BYTES] {
    let header = MessageHeader {
        message_type: message_type::HALT,
        message_length: MESSAGE_HEADER_LENGTH,
        request_id,
        status: status::SUCCESS,
    };
    header.encode()
}

/// The command that carries an RNDIS message to the device.
///
/// RNDIS rides on CDC's encapsulated commands: the message is the data
/// stage of the request, so the request only has to carry its length.
#[must_use]
pub fn send_message(message: &[u8], interface: u16) -> Option<ClassRequest> {
    cdc::send_encapsulated_command(message, interface)
}

/// The request that reads the device's answer to an encapsulated command.
#[must_use]
pub fn read_response(length: u16, interface: u16) -> ClassRequest {
    cdc::get_encapsulated_response(length, interface)
}

/// What a driver asks the device to do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command {
    /// The initialization message.
    Init,
    /// A query of an object identifier.
    Query,
    /// A set of an object identifier.
    Set,
    /// The message that shuts the device down.
    Halt,
    /// A frame.
    Packet,
}

/// The state a device is in.
///
/// The states are the ones the protocol has: a device that has not been
/// initialized cannot be asked anything, one that has been initialized can
/// be queried, and one that carries frames is in the third state. A
/// driver that sends a frame before the initialization has been answered
/// is talking to a device that is not listening.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum State {
    /// The device has not been initialized.
    #[default]
    Uninitialized,
    /// The device has been initialized, and can be queried.
    Initialized,
    /// The device carries frames.
    DataCarrying,
}

impl State {
    /// The state the device is in after it is sent `command`.
    ///
    /// Returns `None` if the command does not fit the state: a device
    /// that has not been initialized cannot be queried, and a device that
    /// has been initialized cannot carry a frame until it has said it is
    /// ready.
    #[must_use]
    pub fn next(self, command: Command) -> Option<Self> {
        match (self, command) {
            (Self::Uninitialized, Command::Init) => Some(Self::Initialized),
            (Self::Initialized | Self::DataCarrying, Command::Init) => Some(Self::Initialized),
            (Self::Uninitialized, _) => None,
            (Self::Initialized, Command::Query | Command::Set) => Some(Self::Initialized),
            (Self::Initialized, Command::Packet) => Some(Self::DataCarrying),
            (Self::Initialized, Command::Halt) => Some(Self::Uninitialized),
            (Self::DataCarrying, Command::Packet) => Some(Self::DataCarrying),
            (Self::DataCarrying, Command::Query | Command::Set) => Some(Self::DataCarrying),
            (Self::DataCarrying, Command::Halt) => Some(Self::Uninitialized),
        }
    }
}

/// The answer to the request `request_id`, among the replies read so far.
///
/// A device answers a request with a message that carries the completion
/// bit and the request's number, and the driver has one request in flight
/// at a time in practice, so the match is exact.
#[must_use]
pub fn find_reply(replies: &[MessageHeader], request_id: u32) -> Option<&MessageHeader> {
    replies.iter().find(|reply| reply.is_reply_to(request_id))
}
