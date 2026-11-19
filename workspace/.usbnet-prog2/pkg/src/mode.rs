//! Bringing the device up, and the framing of what it carries.
//!
//! A USB Ethernet dongle is a CDC device first: the class requests that
//! bring it up, then the subclass's own framing of the frames that ride on
//! the bulk endpoints. What those requests are, and how a frame is
//! wrapped, is `xaqu-usbnet`'s knowledge; what is here is the order the
//! requests go out in, and the bytes this driver's own buffers hold.

use xaqu_usb_serial::{Setup, Usb};
use xaqu_usbnet::cdc::{self, ClassRequest};
use xaqu_usbnet::ecm;
use xaqu_usbnet::ncm;

/// The class request type of a request the device answers: the class one
/// the other way round, which `xaqu-usbnet`'s requests do not carry,
/// because they are all the host's to send.
const CLASS_REQUEST_IN: u8 = 0xC1;
/// `GET_DESCRIPTOR`: the standard request that reads a descriptor.
const GET_DESCRIPTOR: u8 = 6;
/// The configuration descriptor, as `wValue` names it.
const CONFIGURATION: u16 = 0x0100;
/// The descriptor type of a class-specific interface descriptor.
const CS_INTERFACE: u8 = 0x0F;
/// The subtype of the Ethernet functional descriptor: the one that holds
/// the station's address.
const ETHERNET_FUNCTIONAL: u8 = 0x01;
/// How large an input block may be: as much as one OUT slot holds, so a
/// block never has to be split over two transfers.
pub(crate) const NTB_INPUT_SIZE: u16 = 4096;
/// How large one datagram may be: an Ethernet frame with its header.
pub(crate) const MAX_DATAGRAM_SIZE: u16 = 2048;
/// The most a block may be, as this driver reads it.
pub(crate) const MAX_BLOCK: usize = 4096;
/// The most datagrams one block may name.
const MAX_DATAGRAMS: u16 = 64;

/// The protocol a dongle speaks, as its subclass says.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Protocol {
    /// The Ethernet Control Model: one frame per bulk transfer.
    Ecm,
    /// The Network Control Model: frames batched into blocks.
    Ncm,
}

impl Protocol {
    /// The protocol the subclass `subclass` names.
    pub(crate) fn of(subclass: u8) -> Option<Self> {
        match subclass {
            cdc::subclass::ETHERNET => Some(Self::Ecm),
            cdc::subclass::NCM => Some(Self::Ncm),
            _ => None,
        }
    }

    /// The name the console says the protocol by.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Ecm => "ECM",
            Self::Ncm => "NCM",
        }
    }
}

/// Why the device would not come up.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ModeError;

/// Bring the device up, by subclass: ECM's three class requests, or NCM's
/// block setup. The station's address is ECM's, and is read out of the
/// configuration descriptor.
pub(crate) fn bring_up(
    usb: &mut impl Usb,
    interface: u16,
    protocol: Protocol,
) -> Result<Option<[u8; 6]>, ModeError> {
    match protocol {
        Protocol::Ecm => {
            ecm_up(usb, interface)?;
            read_mac(usb)
        }
        Protocol::Ncm => {
            ncm_up(usb, interface)?;
            Ok(None)
        }
    }
}

/// ECM's bring-up: the packet filter, the multicast bitmap, and the
/// control line state.
///
/// The order is the kernel's: a device with no filter set may drop the very
/// frames the multicast bitmap is about to ask for, and the control line
/// state is the device's own "the link is up" switch, which comes last.
fn ecm_up(usb: &mut impl Usb, interface: u16) -> Result<(), ModeError> {
    let setup = ecm::Setup::standard();
    for request in setup.requests(interface) {
        // The second request's data stage is the multicast bitmap; the
        // other two have none.
        let stage: &[u8] = if request.request == cdc::request::SET_ETHERNET_MULTICAST_FILTERS {
            setup.multicast_filter_bytes()
        } else {
            &[]
        };
        send_class(usb, &request, stage)?;
    }
    Ok(())
}

/// NCM's bring-up: what the device's blocks may be, and the format they are
/// in.
///
/// The parameters are asked for first, because the device's answer says how
/// large its blocks may be; the format is then the 16-byte-header one,
/// which every device has; and the sizes are what this driver reads and
/// writes, so that a block never has to be split.
fn ncm_up(usb: &mut impl Usb, interface: u16) -> Result<(), ModeError> {
    let mut parameters = ncm::get_ntb_parameters(interface);
    // The request the crate makes carries no length, because a request it
    // does not read has none; this one is read, so it asks for the
    // parameters' own length.
    parameters.length = u16::try_from(ncm::NCM_PARAMETERS_BYTES).unwrap_or(0);
    let setup = Setup {
        request_type: CLASS_REQUEST_IN,
        request: parameters.request,
        value: parameters.value,
        index: parameters.index,
        length: parameters.length,
    };
    let mut buffer = [0_u8; CONTROL_BYTES];
    control(usb, &setup, &mut buffer)?;
    send_class(usb, &ncm::set_ntb_format(ncm::NTB16_FORMAT, interface), &[])?;
    send_class(
        usb,
        &ncm::set_ntb_input_size(NTB_INPUT_SIZE, interface),
        &[],
    )?;
    send_class(
        usb,
        &ncm::set_max_datagram_size(MAX_DATAGRAM_SIZE, interface),
        &[],
    )?;
    Ok(())
}

/// The station's address, as the configuration descriptor carries it.
///
/// The descriptor is read in two steps: its first nine bytes say how long
/// the whole of it is, and then it is read whole, for the functional
/// descriptor that holds the address.
fn read_mac(usb: &mut impl Usb) -> Result<Option<[u8; 6]>, ModeError> {
    let (head, read) = read_descriptor(usb, 9)?;
    let total = usize::from(u16::from_le_bytes([
        *head.get(2).unwrap_or(&0),
        *head.get(3).unwrap_or(&0),
    ]));
    let total = total.clamp(9, CONTROL_BYTES);
    let (descriptor, _) = read_descriptor(usb, total)?;
    let _ = read;
    Ok(ethernet_mac(&descriptor))
}

/// The configuration descriptor, and how many bytes of it were read.
fn read_descriptor(
    usb: &mut impl Usb,
    length: usize,
) -> Result<([u8; CONTROL_BYTES], usize), ModeError> {
    let setup = Setup {
        request_type: 0x80,
        request: GET_DESCRIPTOR,
        value: CONFIGURATION,
        index: 0,
        length: u16::try_from(length).unwrap_or(0),
    };
    let mut buffer = [0_u8; CONTROL_BYTES];
    let read = control(usb, &setup, &mut buffer)?;
    Ok((buffer, read))
}

/// The station's address, out of the Ethernet functional descriptor.
///
/// The configuration descriptor is a list of descriptors, each with its
/// length and its type; the functional one of the Ethernet type holds the
/// address in the six bytes after its subtype.
fn ethernet_mac(descriptor: &[u8]) -> Option<[u8; 6]> {
    let mut at = 0;
    while at < descriptor.len() {
        let length = usize::from(*descriptor.get(at)?);
        let kind = *descriptor.get(at + 1)?;
        let subtype = *descriptor.get(at + 2)?;
        if kind == CS_INTERFACE && subtype == ETHERNET_FUNCTIONAL {
            return mac_from_functional(descriptor.get(at + 3..at + 9)?);
        }
        if length == 0 {
            break;
        }
        at += length;
    }
    None
}

/// The station's address, as `xaqu-usbnet` reads it.
///
/// The crate's reader takes the descriptor in its own shape: its length,
/// its type, and the six address bytes. The functional descriptor the
/// device sends holds the same six bytes, after its subtype, so the shape
/// the crate reads is made of them.
fn mac_from_functional(address: &[u8]) -> Option<[u8; 6]> {
    let mut shaped = [0_u8; ecm::MAC_DESCRIPTOR_BYTES];
    let slot = shaped.get_mut(..ecm::MAC_DESCRIPTOR_BYTES)?;
    let first = slot.first_mut()?;
    *first = u8::try_from(ecm::MAC_DESCRIPTOR_BYTES).unwrap_or(0);
    let second = slot.get_mut(1)?;
    *second = ecm::ETHERNET_DESCRIPTOR_TYPE;
    let rest = slot.get_mut(2..)?;
    rest.copy_from_slice(address.get(..rest.len())?);
    ecm::mac_from_descriptor(&shaped)
}

/// The frame a bulk transfer carries, in ECM: the transfer's bytes.
pub(crate) fn ecm_frame(bytes: &[u8]) -> Option<&[u8]> {
    ecm::frame_from_transfer(bytes)
}

/// The frames a block carries, in NCM: its datagrams, in order.
///
/// A device may put a message header in front of the block header, which
/// the crate finds; the datagrams are then the block's, one by one.
pub(crate) fn ncm_frames(block: &[u8], mut deliver: impl FnMut(&[u8])) {
    let block = ncm::block_after_message_header(block).unwrap_or(block);
    for ordinal in 0..MAX_DATAGRAMS {
        let Some(frame) = ncm::find_datagram(block, ordinal) else {
            break;
        };
        if frame.is_empty() {
            continue;
        }
        deliver(frame);
    }
}

/// The block a frame goes out in: the 16-byte-header block header, the
/// frame itself, and the pointer list that says where the frame is.
///
/// The list is sixteen bytes: the crate's reader wants at least that, and
/// the entry past the frame's is empty, which a device reads as a
/// datagram of no length and drops.
pub(crate) fn ncm_block(frame: &[u8], sequence: u16, into: &mut [u8]) -> Option<usize> {
    let padded = frame.len().div_ceil(ncm::NDP_ALIGN_MIN) * ncm::NDP_ALIGN_MIN;
    let ndp_at = ncm::NTB16_HEADER_BYTES + padded;
    let ndp_len = ncm::Ndp16Header::BYTES + ncm::Dpe16::BYTES;
    let block_len = ndp_at + ndp_len;
    if into.len() < block_len {
        return None;
    }
    let header = ncm::Ntb16Header::new(
        sequence,
        u16::try_from(block_len).ok()?,
        u16::try_from(ndp_at).ok()?,
    );
    into.get_mut(..ncm::NTB16_HEADER_BYTES)?
        .copy_from_slice(&header.encode());
    into.get_mut(ncm::NTB16_HEADER_BYTES..ndp_at)?
        .copy_from_slice(frame);
    // The padding between the frame and the list is the block's own.
    let padding = ncm::NTB16_HEADER_BYTES + frame.len()..ndp_at;
    into.get_mut(padding)?.fill(0);
    let list = ncm_list(frame, ndp_len)?;
    into.get_mut(ndp_at..block_len)?.copy_from_slice(&list);
    Some(block_len)
}

/// The pointer list of a block with one datagram in it.
///
/// The list is the signature, where the next list is, how long this one
/// is, and the entry: where the datagram is in the block, and how long it
/// is.
fn ncm_list(frame: &[u8], ndp_len: usize) -> Option<[u8; 16]> {
    let mut list = [0_u8; 16];
    if ndp_len != list.len() {
        return None;
    }
    list.get_mut(..4)?.copy_from_slice(&ncm::NDP16_SIGNATURE_0.to_le_bytes());
    list.get_mut(4..6)?.copy_from_slice(&0_u16.to_le_bytes());
    list.get_mut(6..8)?.copy_from_slice(&u16::try_from(ndp_len).ok()?.to_le_bytes());
    list.get_mut(8..10)?
        .copy_from_slice(&u16::try_from(ncm::NTB16_HEADER_BYTES).ok()?.to_le_bytes());
    list.get_mut(10..12)?.copy_from_slice(&u16::try_from(frame.len()).ok()?.to_le_bytes());
    Some(list)
}

/// A class request the host sends the device.
fn send_class(
    usb: &mut impl Usb,
    request: &ClassRequest,
    stage: &[u8],
) -> Result<(), ModeError> {
    let setup = Setup {
        request_type: cdc::CLASS_REQUEST_TYPE,
        request: request.request,
        value: request.value,
        index: request.index,
        length: request.length,
    };
    let mut buffer = [0_u8; CONTROL_BYTES];
    let len = stage.len().min(usize::from(request.length));
    if let Some(slot) = buffer.get_mut(..len) {
        slot.copy_from_slice(stage.get(..len).unwrap_or_default());
    }
    control(usb, &setup, &mut buffer)?;
    Ok(())
}

/// A control transfer, whatever it failed with.
fn control(usb: &mut impl Usb, setup: &Setup, data: &mut [u8]) -> Result<usize, ModeError> {
    usb.control(setup, data).map_err(|_| ModeError)
}
