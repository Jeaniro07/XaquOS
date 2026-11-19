//! The Ethernet Control Model: CDC-ECM, the simplest of the three dongle
//! protocols.
//!
//! ECM carries one Ethernet frame per bulk transfer, in each direction: a
//! frame goes out as one bulk OUT transfer, and a frame comes in as one
//! bulk IN transfer. There is no batching, no length prefix and no block
//! header: the transfer's length is the frame's length, and the driver's
//! Ethernet layer is handed the transfer's bytes as the frame.
//!
//! Bringing the device up is three control requests, and the order
//! matters. The packet filter is set first, because a device with no
//! filter set may drop the very frames the multicast bitmap is about to
//! ask for; then the multicast bitmap, which is a bitmap of the 1024
//! multicast addresses the hardware can filter on; and last the control
//! line state, which is the device's own "the link is up" switch.
//! `drivers/net/usb/cdc_ether.c` sends them in that order.

use crate::cdc::{self, ClassRequest};
use crate::bytes::get16_be;

/// The bytes of the multicast filter bitmap: 128 bytes, one bit per
/// multicast address, of which the device has 1024.
pub const MULTICAST_FILTER_BYTES: usize = 128;

/// The length of the multicast filter bitmap, as the request carries it.
pub const MULTICAST_FILTER_LENGTH: u16 = 128;

/// The bytes of an Ethernet frame's header: the destination address, the
/// source address and the type.
pub const FRAME_HEADER_BYTES: usize = 14;

/// The bytes of the Ethernet functional descriptor that carries the
/// station's address: the length, the type and then six address bytes.
pub const MAC_DESCRIPTOR_BYTES: usize = 8;

/// The functional descriptor type of the Ethernet descriptor.
pub const ETHERNET_DESCRIPTOR_TYPE: u8 = 0x06;

/// The setup a driver sends to bring an ECM device up.
///
/// The filter bits are the ones ECM needs: everything the device's
/// hardware can filter, so that the software above sees every frame the
/// station is meant to see.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Setup {
    /// Which frames the device passes up.
    pub packet_filter: u16,
    /// Which multicast addresses the device passes up.
    pub multicast_filters: [u8; MULTICAST_FILTER_BYTES],
    /// Which of DTR and RTS are raised.
    pub control_line_state: u16,
}

impl Setup {
    /// The setup that asks for every frame the station is meant to see.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            packet_filter: cdc::packet_filter::PROMISCUOUS
                | cdc::packet_filter::ALL_MULTICAST
                | cdc::packet_filter::DIRECTED
                | cdc::packet_filter::BROADCAST
                | cdc::packet_filter::MULTICAST,
            multicast_filters: [0; MULTICAST_FILTER_BYTES],
            control_line_state: cdc::control_line_state::DTR | cdc::control_line_state::RTS,
        }
    }

    /// The three requests the setup is, in the order they go out.
    ///
    /// The second request's data stage is the multicast bitmap, which the
    /// caller has to send with it; the request itself only says how long
    /// that stage is.
    #[must_use]
    pub fn requests(&self, interface: u16) -> [ClassRequest; 3] {
        [
            cdc::set_packet_filter(self.packet_filter, interface),
            ClassRequest::new(
                cdc::request::SET_ETHERNET_MULTICAST_FILTERS,
                0,
                interface,
                MULTICAST_FILTER_LENGTH,
            ),
            cdc::set_control_line_state(self.control_line_state, interface),
        ]
    }

    /// The multicast bitmap the setup sends as the data stage of its
    /// second request.
    #[must_use]
    pub fn multicast_filter_bytes(&self) -> &[u8; MULTICAST_FILTER_BYTES] {
        &self.multicast_filters
    }
}

/// The one Ethernet frame a bulk transfer carries.
///
/// ECM has no framing of its own: the transfer's bytes are the frame, so
/// the only thing to check is that there are enough of them for the frame
/// header to exist. A short transfer is a device that sent nothing
/// usable, and the driver must not hand it to the Ethernet layer.
#[must_use]
pub fn frame_from_transfer(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < FRAME_HEADER_BYTES {
        return None;
    }
    Some(bytes)
}

/// The station's address, as the Ethernet functional descriptor carries it.
///
/// The descriptor is the one the device's configuration descriptor holds:
/// its length, its type, then the six address bytes. A device that has no
/// such descriptor has no address of its own, and the driver has to fall
/// back on the address the bus gave it.
#[must_use]
pub fn mac_from_descriptor(bytes: &[u8]) -> Option<[u8; 6]> {
    let head = bytes.get(..MAC_DESCRIPTOR_BYTES)?;
    if *head.get(1)? != ETHERNET_DESCRIPTOR_TYPE {
        return None;
    }
    let mut address = [0_u8; 6];
    for (index, slot) in address.iter_mut().enumerate() {
        *slot = *head.get(index + 2)?;
    }
    Some(address)
}

/// The type field of an Ethernet frame, as the frame header's third field.
///
/// The type is in network order, which is the one field of the Ethernet
/// frame that is not little-endian.
#[must_use]
pub fn frame_ethertype(frame: &[u8]) -> Option<u16> {
    get16_be(frame, 12)
}
