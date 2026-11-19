//! The Communications Device Class: the requests and the notifications that
//! every USB Ethernet dongle answers before it carries a frame.
//!
//! A dongle that speaks CDC-ECM, CDC-NCM or RNDIS is a CDC device first.
//! The class's interface control endpoint takes the requests
//! `include/uapi/linux/usb/cdc.h` numbers, and the interface's interrupt
//! endpoint carries the notifications the device sends unasked: that the
//! link came up, that a response to an encapsulated command is waiting, or
//! that the serial line's state changed.
//!
//! The three protocols differ in what rides on the bulk endpoints, not in
//! how the device is brought up, so the requests live here and the
//! protocols are written on top of them.

use crate::bytes::{get16, get32, put16, put32};

/// The class request codes, as `include/uapi/linux/usb/cdc.h` numbers them.
pub mod request {
    /// `SEND_ENCAPSULATED_COMMAND`: send the device a command its own
    /// protocol defines, in the data stage.
    pub const SEND_ENCAPSULATED_COMMAND: u8 = 0x00;
    /// `GET_ENCAPSULATED_RESPONSE`: read the device's answer to one.
    pub const GET_ENCAPSULATED_RESPONSE: u8 = 0x01;
    /// `SET_LINE_CODING`: set the serial line's rate, framing and parity.
    pub const SET_LINE_CODING: u8 = 0x20;
    /// `GET_LINE_CODING`: read the serial line's rate, framing and parity.
    pub const GET_LINE_CODING: u8 = 0x21;
    /// `SET_CONTROL_LINE_STATE`: raise or lower DTR and RTS.
    pub const SET_CONTROL_LINE_STATE: u8 = 0x22;
    /// `SET_ETHERNET_MULTICAST_FILTERS`: set which multicast addresses the
    /// device passes up.
    pub const SET_ETHERNET_MULTICAST_FILTERS: u8 = 0x40;
    /// `SET_ETHERNET_PACKET_FILTER`: set which frames the device passes up.
    pub const SET_ETHERNET_PACKET_FILTER: u8 = 0x43;
    /// `GET_NTB_PARAMETERS`: ask the device how large its blocks may be.
    pub const GET_NTB_PARAMETERS: u8 = 0x80;
    /// `SET_NTB_FORMAT`: choose the 16- or the 32-byte block header.
    pub const SET_NTB_FORMAT: u8 = 0x84;
    /// `SET_NTB_INPUT_SIZE`: say how large an input block may be.
    pub const SET_NTB_INPUT_SIZE: u8 = 0x86;
    /// `SET_MAX_DATAGRAM_SIZE`: say how large one datagram may be.
    pub const SET_MAX_DATAGRAM_SIZE: u8 = 0x88;
    /// `SET_CRC_MODE`: say how the device checks the frames it sends.
    pub const SET_CRC_MODE: u8 = 0x8A;
}

/// The subclass numbers, as `include/uapi/linux/usb/cdc.h` numbers them.
pub mod subclass {
    /// ETHERNET: the Ethernet Control Model.
    pub const ETHERNET: u8 = 0x06;
    /// EEM: the Ethernet Emulation Mode.
    pub const EEM: u8 = 0x0C;
    /// NCM: the Network Control Model.
    pub const NCM: u8 = 0x0D;
    /// MBIM: the Mobile Broadband Interface Model.
    pub const MBIM: u8 = 0x0E;
}

/// The bits of the Ethernet packet filter, as `cdc.h` defines them.
pub mod packet_filter {
    /// PROMISCUOUS: every frame the device sees, addressed or not.
    pub const PROMISCUOUS: u16 = 1 << 0;
    /// `ALL_MULTICAST`: every multicast frame.
    pub const ALL_MULTICAST: u16 = 1 << 1;
    /// DIRECTED: frames addressed to this station's address.
    pub const DIRECTED: u16 = 1 << 2;
    /// BROADCAST: broadcast frames.
    pub const BROADCAST: u16 = 1 << 3;
    /// MULTICAST: the frames the multicast filter bitmap selects.
    pub const MULTICAST: u16 = 1 << 4;
}

/// The bits of the control line state, as `cdc.h` defines them.
pub mod control_line_state {
    /// DTR: the data terminal ready line, which for an Ethernet dongle is
    /// the device's own "I am up" switch.
    pub const DTR: u16 = 1 << 0;
    /// RTS: the request to send line.
    pub const RTS: u16 = 1 << 1;
}

/// The notification codes the interrupt endpoint carries.
pub mod notification {
    /// `NETWORK_CONNECTION`: the link came up or went down.
    pub const NETWORK_CONNECTION: u8 = 0x00;
    /// `RESPONSE_AVAILABLE`: an answer to an encapsulated command is waiting.
    pub const RESPONSE_AVAILABLE: u8 = 0x01;
    /// `SERIAL_STATE`: the serial line's state changed.
    pub const SERIAL_STATE: u8 = 0x20;
    /// `SPEED_CHANGE`: the link's speed changed.
    pub const SPEED_CHANGE: u8 = 0x2A;
}

/// The request type a class request carries: to the class, to the
/// interface, from the host.
pub const CLASS_REQUEST_TYPE: u8 = 0x21;

/// A class request's setup packet.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ClassRequest {
    /// `bRequest`: the class request code.
    pub request: u8,
    /// `wValue`: what the request sets.
    pub value: u16,
    /// `wIndex`: the interface the request is addressed to.
    pub index: u16,
    /// `wLength`: how many bytes the data stage carries.
    pub length: u16,
}

impl ClassRequest {
    /// The bytes a setup packet takes.
    pub const BYTES: usize = 8;

    /// A request of `request`, addressed to `index`.
    #[must_use]
    pub const fn new(request: u8, value: u16, index: u16, length: u16) -> Self {
        Self {
            request,
            value,
            index,
            length,
        }
    }

    /// The setup packet's bytes, as the bus carries them.
    #[must_use]
    pub const fn bytes(&self) -> [u8; Self::BYTES] {
        let value = self.value.to_le_bytes();
        let index = self.index.to_le_bytes();
        let length = self.length.to_le_bytes();
        [
            CLASS_REQUEST_TYPE,
            self.request,
            value[0],
            value[1],
            index[0],
            index[1],
            length[0],
            length[1],
        ]
    }

    /// The request the setup packet's bytes carry.
    ///
    /// Returns `None` if the bytes are not a class request at all: the
    /// request type has to be the class one, because a driver that reads a
    /// vendor request as a class one would send the wrong thing to the
    /// device.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let head = bytes.get(..Self::BYTES)?;
        if *head.first()? != CLASS_REQUEST_TYPE {
            return None;
        }
        Some(Self {
            request: *head.get(1)?,
            value: get16(head, 2)?,
            index: get16(head, 4)?,
            length: get16(head, 6)?,
        })
    }
}

/// The notification the interrupt endpoint carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Notification {
    /// `bmRequestType`: the type the notification's request carries.
    pub request_type: u8,
    /// `bRequest`: the notification code.
    pub request: u8,
    /// `wValue`: what the notification says.
    pub value: u16,
    /// `wIndex`: the interface the notification is about.
    pub index: u16,
    /// `wLength`: how many bytes follow the notification.
    pub length: u16,
}

impl Notification {
    /// The bytes a notification takes, as `struct cdc_notification` is
    /// packed: eight, with no padding.
    pub const BYTES: usize = 8;

    /// The notification's bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; Self::BYTES] {
        let mut out = [0_u8; Self::BYTES];
        out[0] = self.request_type;
        out[1] = self.request;
        put16(&mut out, 2, self.value);
        put16(&mut out, 4, self.index);
        put16(&mut out, 6, self.length);
        out
    }

    /// The notification the bytes carry.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let head = bytes.get(..Self::BYTES)?;
        Some(Self {
            request_type: *head.first()?,
            request: *head.get(1)?,
            value: get16(head, 2)?,
            index: get16(head, 4)?,
            length: get16(head, 6)?,
        })
    }

    /// Whether the notification says the link came up or went down.
    ///
    /// The value carries the state: zero is disconnected, one connected.
    #[must_use]
    pub fn is_connection_change(&self) -> bool {
        self.request == notification::NETWORK_CONNECTION
    }

    /// Whether the notification says an answer is waiting to be read.
    #[must_use]
    pub fn is_response_available(&self) -> bool {
        self.request == notification::RESPONSE_AVAILABLE
    }
}

/// The serial line's coding, as `struct cdc_line_coding` is packed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct LineCoding {
    /// `dwDTERate`: the bits per second.
    pub dte_rate: u32,
    /// `bCharFormat`: the stop bits.
    pub char_format: u8,
    /// `bParityType`: the parity.
    pub parity_type: u8,
    /// `bDataBits`: the data bits.
    pub data_bits: u8,
}

impl LineCoding {
    /// The bytes a line coding takes, as `struct cdc_line_coding` is
    /// packed: seven, with no padding.
    pub const BYTES: usize = 7;

    /// The line coding's bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; Self::BYTES] {
        let mut out = [0_u8; Self::BYTES];
        put32(&mut out, 0, self.dte_rate);
        out[4] = self.char_format;
        out[5] = self.parity_type;
        out[6] = self.data_bits;
        out
    }

    /// The line coding the bytes carry.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let head = bytes.get(..Self::BYTES)?;
        Some(Self {
            dte_rate: get32(head, 0)?,
            char_format: *head.get(4)?,
            parity_type: *head.get(5)?,
            data_bits: *head.get(6)?,
        })
    }
}

/// The request that says which frames the device passes up.
#[must_use]
pub fn set_packet_filter(filter: u16, interface: u16) -> ClassRequest {
    ClassRequest::new(
        request::SET_ETHERNET_PACKET_FILTER,
        filter,
        interface,
        0,
    )
}

/// The request that raises or lowers DTR and RTS.
#[must_use]
pub fn set_control_line_state(state: u16, interface: u16) -> ClassRequest {
    ClassRequest::new(
        request::SET_CONTROL_LINE_STATE,
        state,
        interface,
        0,
    )
}

/// The request that sets the multicast filter bitmap.
///
/// The bitmap itself is the data stage, so the request only carries how
/// many bytes it has. A bitmap too long for the device's `wLength` is a
/// request the device would truncate, so it is refused here.
#[must_use]
pub fn set_ethernet_multicast_filters(
    filters: &[u8],
    interface: u16,
) -> Option<ClassRequest> {
    let length = u16::try_from(filters.len()).ok()?;
    Some(ClassRequest::new(
        request::SET_ETHERNET_MULTICAST_FILTERS,
        0,
        interface,
        length,
    ))
}

/// The request that sends the device a command its own protocol defines.
///
/// The command is the data stage, so the request only carries its length.
#[must_use]
pub fn send_encapsulated_command(command: &[u8], interface: u16) -> Option<ClassRequest> {
    let length = u16::try_from(command.len()).ok()?;
    Some(ClassRequest::new(
        request::SEND_ENCAPSULATED_COMMAND,
        0,
        interface,
        length,
    ))
}

/// The request that reads the device's answer to an encapsulated command.
#[must_use]
pub fn get_encapsulated_response(length: u16, interface: u16) -> ClassRequest {
    ClassRequest::new(
        request::GET_ENCAPSULATED_RESPONSE,
        0,
        interface,
        length,
    )
}
