//! The Network Control Model: CDC-NCM, the protocol that batches Ethernet
//! frames instead of sending them one at a time.
//!
//! A modern USB Ethernet dongle does not send one frame per bulk transfer,
//! because the overhead of a transfer per frame is larger than the frame.
//! NCM sends a block: a header saying how long the block is and where its
//! datagram list is, the datagrams themselves, and a list at the end of
//! pointers saying where each datagram is and how long it is.
//!
//! The block is either a 16-byte-header block or a 32-byte-header block,
//! chosen with `SET_NTB_FORMAT`, and the datagram list at the end is
//! either a 16-byte-header list or a 32-byte-header list, chosen by the
//! signature the list itself carries. The two versions exist because the
//! 16-byte one cannot address a block larger than 64 KiB and the 32-byte
//! one can; `drivers/net/usb/cdc_ncm.c` prefers the 32-byte one and falls
//! back on the 16-byte one for the devices that only do that.
//!
//! The signatures, the header layouts and the minimum lengths are the
//! kernel's own, as `include/uapi/linux/usb/cdc.h` and
//! `drivers/net/usb/cdc_ncm.c` read them.

use crate::bytes::{get16, get32, put16, put32};
use crate::cdc::{self, ClassRequest};

/// The signature of a 16-byte-header transfer block: "NCMH".
pub const NTB16_SIGNATURE: u32 = 0x484D_434E;

/// The signature of a 32-byte-header transfer block: "ncmh".
pub const NTB32_SIGNATURE: u32 = 0x686D_636E;

/// The signature of the first 16-byte datagram pointer list: "NCM0".
pub const NDP16_SIGNATURE_0: u32 = 0x304D_434E;

/// The signature of a second 16-byte datagram pointer list: "NCM1".
pub const NDP16_SIGNATURE_1: u32 = 0x314D_434E;

/// The signature of the first 32-byte datagram pointer list: "ncm0".
pub const NDP32_SIGNATURE_0: u32 = 0x306D_636E;

/// The signature of a second 32-byte datagram pointer list: "ncm1".
pub const NDP32_SIGNATURE_1: u32 = 0x316D_636E;

/// The protocol a device speaks: the network control model.
pub const NCM_PROTO_NTB: u8 = 1;

/// The block format with the 16-byte header.
pub const NTB16_FORMAT: u16 = 0x00;

/// The block format with the 32-byte header.
pub const NTB32_FORMAT: u16 = 0x01;

/// The alignment every datagram and every pointer list is padded to.
pub const NDP_ALIGN_MIN: usize = 4;

/// The shortest a 16-byte pointer list may be.
pub const NDP16_LENGTH_MIN: u16 = 0x10;

/// The shortest a 32-byte pointer list may be.
pub const NDP32_LENGTH_MIN: u32 = 0x20;

/// The bytes a 16-byte-header block's header takes.
pub const NTB16_HEADER_BYTES: usize = 12;

/// The length a 16-byte-header block's header says it has.
pub const NTB16_HEADER_LENGTH: u16 = 12;

/// The bytes a 32-byte-header block's header takes.
pub const NTB32_HEADER_BYTES: usize = 16;

/// The length a 32-byte-header block's header says it has.
pub const NTB32_HEADER_LENGTH: u16 = 16;

/// The bytes the message header takes, when a device puts one in front of
/// the block header.
pub const NCM_MESSAGE_HEADER_BYTES: usize = 16;

/// The bytes a device's answer to `GET_NTB_PARAMETERS` takes, as
/// `drivers/net/usb/cdc_ncm.c` reads it.
pub const NCM_PARAMETERS_BYTES: usize = 28;

/// The header of a 16-byte-header transfer block.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ntb16Header {
    /// `dwSignature`: always the 16-byte block's signature.
    pub signature: u32,
    /// `wHeaderLength`: how long the header is, from here to the first
    /// datagram.
    pub header_length: u16,
    /// `wSequenceNumber`: the block's number, so a driver can tell blocks
    /// it has seen from blocks it has not.
    pub sequence_number: u16,
    /// `wBlockLength`: how long the whole block is, header and pointer
    /// list included.
    pub block_length: u16,
    /// `wNdpIndex`: where in the block the pointer list is.
    pub ndp_index: u16,
}

impl Ntb16Header {
    /// The bytes the header takes.
    pub const BYTES: usize = NTB16_HEADER_BYTES;

    /// The header of block `sequence_number`.
    #[must_use]
    pub const fn new(sequence_number: u16, block_length: u16, ndp_index: u16) -> Self {
        Self {
            signature: NTB16_SIGNATURE,
            header_length: NTB16_HEADER_LENGTH,
            sequence_number,
            block_length,
            ndp_index,
        }
    }

    /// The header's bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; Self::BYTES] {
        let mut out = [0_u8; Self::BYTES];
        put32(&mut out, 0, self.signature);
        put16(&mut out, 4, self.header_length);
        put16(&mut out, 6, self.sequence_number);
        put16(&mut out, 8, self.block_length);
        put16(&mut out, 10, self.ndp_index);
        out
    }

    /// The header the bytes carry.
    ///
    /// Returns `None` if the bytes are not a 16-byte-header block's
    /// header: the signature has to be the right one, and the header has
    /// to be long enough to hold the fields.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let head = bytes.get(..Self::BYTES)?;
        let signature = get32(head, 0)?;
        if signature != NTB16_SIGNATURE {
            return None;
        }
        let header_length = get16(head, 4)?;
        if usize::from(header_length) < Self::BYTES {
            return None;
        }
        Some(Self {
            signature,
            header_length,
            sequence_number: get16(head, 6)?,
            block_length: get16(head, 8)?,
            ndp_index: get16(head, 10)?,
        })
    }
}

/// The header of a 32-byte-header transfer block.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ntb32Header {
    /// `dwSignature`: always the 32-byte block's signature.
    pub signature: u32,
    /// `wHeaderLength`: how long the header is.
    pub header_length: u16,
    /// `wSequenceNumber`: the block's number.
    pub sequence_number: u16,
    /// `dwBlockLength`: how long the whole block is.
    pub block_length: u32,
    /// `dwNdpIndex`: where in the block the pointer list is.
    pub ndp_index: u32,
}

impl Ntb32Header {
    /// The bytes the header takes.
    pub const BYTES: usize = NTB32_HEADER_BYTES;

    /// The header of block `sequence_number`.
    #[must_use]
    pub const fn new(sequence_number: u16, block_length: u32, ndp_index: u32) -> Self {
        Self {
            signature: NTB32_SIGNATURE,
            header_length: NTB32_HEADER_LENGTH,
            sequence_number,
            block_length,
            ndp_index,
        }
    }

    /// The header's bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; Self::BYTES] {
        let mut out = [0_u8; Self::BYTES];
        put32(&mut out, 0, self.signature);
        put16(&mut out, 4, self.header_length);
        put16(&mut out, 6, self.sequence_number);
        put32(&mut out, 8, self.block_length);
        put32(&mut out, 12, self.ndp_index);
        out
    }

    /// The header the bytes carry.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let head = bytes.get(..Self::BYTES)?;
        let signature = get32(head, 0)?;
        if signature != NTB32_SIGNATURE {
            return None;
        }
        let header_length = get16(head, 4)?;
        if usize::from(header_length) < Self::BYTES {
            return None;
        }
        Some(Self {
            signature,
            header_length,
            sequence_number: get16(head, 6)?,
            block_length: get32(head, 8)?,
            ndp_index: get32(head, 12)?,
        })
    }
}

/// The header of a 16-byte datagram pointer list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ndp16Header {
    /// `dwSignature`: the list's signature, which says which version it is.
    pub signature: u32,
    /// `wNextNdpIndex`: where the next list is, for a device that sends
    /// several.
    pub next_ndp_index: u16,
    /// `wTotalLength`: how long the whole list is, header and entries.
    pub total_length: u16,
}

impl Ndp16Header {
    /// The bytes the header takes.
    pub const BYTES: usize = 8;

    /// The header the bytes at `at` carry.
    #[must_use]
    pub fn decode_at(bytes: &[u8], at: usize) -> Option<Self> {
        let head = bytes.get(at..at.checked_add(Self::BYTES)?)?;
        let signature = get32(head, 0)?;
        if signature != NDP16_SIGNATURE_0 && signature != NDP16_SIGNATURE_1 {
            return None;
        }
        Some(Self {
            signature,
            next_ndp_index: get16(head, 4)?,
            total_length: get16(head, 6)?,
        })
    }
}

/// The header of a 32-byte datagram pointer list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ndp32Header {
    /// `dwSignature`: the list's signature, which says which version it is.
    pub signature: u32,
    /// `wNextNdpIndex`: where the next list is.
    pub next_ndp_index: u16,
    /// `wReserved`: unused, and always zero.
    pub reserved: u16,
    /// `dwTotalLength`: how long the whole list is.
    pub total_length: u32,
    /// `dwNextNdpIndex`: where the next list is, in the wide form.
    pub next_ndp_index_wide: u32,
}

impl Ndp32Header {
    /// The bytes the header takes.
    pub const BYTES: usize = 16;

    /// The header the bytes at `at` carry.
    #[must_use]
    pub fn decode_at(bytes: &[u8], at: usize) -> Option<Self> {
        let head = bytes.get(at..at.checked_add(Self::BYTES)?)?;
        let signature = get32(head, 0)?;
        if signature != NDP32_SIGNATURE_0 && signature != NDP32_SIGNATURE_1 {
            return None;
        }
        Some(Self {
            signature,
            next_ndp_index: get16(head, 4)?,
            reserved: get16(head, 6)?,
            total_length: get32(head, 8)?,
            next_ndp_index_wide: get32(head, 12)?,
        })
    }
}

/// One entry of a 16-byte datagram pointer list: where a datagram is in
/// the block, and how long it is.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Dpe16 {
    /// `wDatagramIndex`: where the datagram starts, counted from the
    /// start of the block.
    pub datagram_index: u16,
    /// `wDatagramLength`: how long the datagram is.
    pub datagram_length: u16,
}

impl Dpe16 {
    /// The bytes an entry takes.
    pub const BYTES: usize = 4;

    /// The entry the bytes at `at` carry.
    #[must_use]
    pub fn decode_at(bytes: &[u8], at: usize) -> Option<Self> {
        let head = bytes.get(at..at.checked_add(Self::BYTES)?)?;
        Some(Self {
            datagram_index: get16(head, 0)?,
            datagram_length: get16(head, 2)?,
        })
    }
}

/// One entry of a 32-byte datagram pointer list.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Dpe32 {
    /// `dwDatagramIndex`: where the datagram starts, counted from the
    /// start of the block.
    pub datagram_index: u32,
    /// `dwDatagramLength`: how long the datagram is.
    pub datagram_length: u32,
}

impl Dpe32 {
    /// The bytes an entry takes.
    pub const BYTES: usize = 8;

    /// The entry the bytes at `at` carry.
    #[must_use]
    pub fn decode_at(bytes: &[u8], at: usize) -> Option<Self> {
        let head = bytes.get(at..at.checked_add(Self::BYTES)?)?;
        Some(Self {
            datagram_index: get32(head, 0)?,
            datagram_length: get32(head, 4)?,
        })
    }
}

/// Which of the two block formats a block is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NtbVersion {
    /// The block has the 16-byte header.
    Sixteen,
    /// The block has the 32-byte header.
    ThirtyTwo,
}

/// Which of the two block formats the bytes at the front are.
#[must_use]
pub fn version(bytes: &[u8]) -> Option<NtbVersion> {
    if Ntb16Header::decode(bytes).is_some() {
        return Some(NtbVersion::Sixteen);
    }
    if Ntb32Header::decode(bytes).is_some() {
        return Some(NtbVersion::ThirtyTwo);
    }
    None
}

/// A parsed 16-byte-header block.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ntb16 {
    /// The block's header.
    pub header: Ntb16Header,
    /// How many datagrams the block's pointer list names.
    pub datagram_count: u16,
}

impl Ntb16 {
    /// The block the bytes carry.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let header = Ntb16Header::decode(bytes)?;
        let block_length = usize::from(header.block_length);
        if block_length > bytes.len() {
            return None;
        }
        let count = ndp16_count(bytes, usize::from(header.ndp_index), block_length)?;
        Some(Self {
            header,
            datagram_count: count,
        })
    }

    /// The pointer list's entry for the datagram of number `ordinal`.
    #[must_use]
    pub fn datagram(&self, bytes: &[u8], ordinal: u16) -> Option<Dpe16> {
        if ordinal >= self.datagram_count {
            return None;
        }
        let at = usize::from(self.header.ndp_index)
            + Ndp16Header::BYTES
            + usize::from(ordinal) * Dpe16::BYTES;
        Dpe16::decode_at(bytes, at)
    }

    /// The datagram of number `ordinal`, as the bytes of the frame.
    #[must_use]
    pub fn datagram_bytes<'a>(&self, bytes: &'a [u8], ordinal: u16) -> Option<&'a [u8]> {
        let entry = self.datagram(bytes, ordinal)?;
        let start = usize::from(entry.datagram_index);
        let end = start.checked_add(usize::from(entry.datagram_length))?;
        bytes.get(start..end)
    }
}

/// A parsed 32-byte-header block.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ntb32 {
    /// The block's header.
    pub header: Ntb32Header,
    /// How many datagrams the block's pointer list names.
    pub datagram_count: u32,
}

impl Ntb32 {
    /// The block the bytes carry.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let header = Ntb32Header::decode(bytes)?;
        let block_length = usize::try_from(header.block_length).ok()?;
        if block_length > bytes.len() {
            return None;
        }
        let count = ndp32_count(bytes, usize::try_from(header.ndp_index).ok()?, block_length)?;
        Some(Self {
            header,
            datagram_count: count,
        })
    }

    /// The pointer list's entry for the datagram of number `ordinal`.
    #[must_use]
    pub fn datagram(&self, bytes: &[u8], ordinal: u32) -> Option<Dpe32> {
        if ordinal >= self.datagram_count {
            return None;
        }
        let at = usize::try_from(self.header.ndp_index).ok()?
            + Ndp32Header::BYTES
            + usize::try_from(ordinal).ok()? * Dpe32::BYTES;
        Dpe32::decode_at(bytes, at)
    }

    /// The datagram of number `ordinal`, as the bytes of the frame.
    #[must_use]
    pub fn datagram_bytes<'a>(&self, bytes: &'a [u8], ordinal: u32) -> Option<&'a [u8]> {
        let entry = self.datagram(bytes, ordinal)?;
        let start = usize::try_from(entry.datagram_index).ok()?;
        let end = start.checked_add(usize::try_from(entry.datagram_length).ok()?)?;
        bytes.get(start..end)
    }
}

/// How many datagrams the 16-byte pointer list at `offset` names.
fn ndp16_count(bytes: &[u8], offset: usize, block_length: usize) -> Option<u16> {
    let header = Ndp16Header::decode_at(bytes, offset)?;
    let total = usize::from(header.total_length);
    if total < usize::from(NDP16_LENGTH_MIN) {
        return None;
    }
    if offset.checked_add(total)? > block_length {
        return None;
    }
    let count = total.checked_sub(Ndp16Header::BYTES)? / Dpe16::BYTES;
    u16::try_from(count).ok()
}

/// How many datagrams the 32-byte pointer list at `offset` names.
fn ndp32_count(bytes: &[u8], offset: usize, block_length: usize) -> Option<u32> {
    let header = Ndp32Header::decode_at(bytes, offset)?;
    let total = usize::try_from(header.total_length).ok()?;
    if u32::try_from(total).ok()? < NDP32_LENGTH_MIN {
        return None;
    }
    if offset.checked_add(total)? > block_length {
        return None;
    }
    let count = total.checked_sub(Ndp32Header::BYTES)? / Dpe32::BYTES;
    u32::try_from(count).ok()
}

/// The datagram of number `ordinal` in a block, whichever version the
/// block is.
#[must_use]
pub fn find_datagram(bytes: &[u8], ordinal: u16) -> Option<&[u8]> {
    match version(bytes)? {
        NtbVersion::Sixteen => Ntb16::parse(bytes)?.datagram_bytes(bytes, ordinal),
        NtbVersion::ThirtyTwo => {
            let block = Ntb32::parse(bytes)?;
            block.datagram_bytes(bytes, u32::from(ordinal))
        }
    }
}

/// The message header a device may put in front of the block header.
///
/// The message header says what the block is for -- a datagram, or the
/// block's own parameters -- and how long the whole message is. It is
/// optional: a device that does not use it puts the block header at the
/// front of the transfer.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct NcmMessageHeader {
    /// `dwMessageType`: what the message is.
    pub message_type: u32,
    /// `wHeaderLen`: how long the header is.
    pub header_length: u16,
    /// `wSequenceNumber`: the message's number.
    pub sequence_number: u16,
    /// `wMessageLen`: how long the whole message is.
    pub message_length: u16,
    /// `wNdpIndex`: where in the message the pointer list is.
    pub ndp_index: u16,
    /// The four bytes the header pads itself out to sixteen with.
    pub reserved: u32,
}

impl NcmMessageHeader {
    /// The bytes the header takes.
    pub const BYTES: usize = NCM_MESSAGE_HEADER_BYTES;

    /// The header's bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; Self::BYTES] {
        let mut out = [0_u8; Self::BYTES];
        put32(&mut out, 0, self.message_type);
        put16(&mut out, 4, self.header_length);
        put16(&mut out, 6, self.sequence_number);
        put16(&mut out, 8, self.message_length);
        put16(&mut out, 10, self.ndp_index);
        put32(&mut out, 12, self.reserved);
        out
    }

    /// The header the bytes carry.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let head = bytes.get(..Self::BYTES)?;
        Some(Self {
            message_type: get32(head, 0)?,
            header_length: get16(head, 4)?,
            sequence_number: get16(head, 6)?,
            message_length: get16(head, 8)?,
            ndp_index: get16(head, 10)?,
            reserved: get32(head, 12)?,
        })
    }
}

/// The block that follows a message header, if there is one.
///
/// A transfer either starts with the block header or with the message
/// header in front of it, and the two are told apart by looking at the
/// block header's signature where it would be.
#[must_use]
pub fn block_after_message_header(bytes: &[u8]) -> Option<&[u8]> {
    let rest = bytes.get(NCM_MESSAGE_HEADER_BYTES..)?;
    if version(rest).is_some() {
        return Some(rest);
    }
    None
}

/// The request that asks the device how large its blocks may be.
#[must_use]
pub fn get_ntb_parameters(interface: u16) -> ClassRequest {
    ClassRequest::new(cdc::request::GET_NTB_PARAMETERS, 0, interface, 0)
}

/// The request that chooses the block format.
///
/// The format is the 16- or the 32-byte header, and the device says which
/// it prefers in the answer to `GET_NTB_PARAMETERS`.
#[must_use]
pub fn set_ntb_format(format: u16, interface: u16) -> ClassRequest {
    ClassRequest::new(cdc::request::SET_NTB_FORMAT, format, interface, 0)
}

/// The request that says how large an input block may be.
#[must_use]
pub fn set_ntb_input_size(size: u16, interface: u16) -> ClassRequest {
    ClassRequest::new(cdc::request::SET_NTB_INPUT_SIZE, size, interface, 0)
}

/// The request that says how large one datagram may be.
#[must_use]
pub fn set_max_datagram_size(size: u16, interface: u16) -> ClassRequest {
    ClassRequest::new(cdc::request::SET_MAX_DATAGRAM_SIZE, size, interface, 0)
}
