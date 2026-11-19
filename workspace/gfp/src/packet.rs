//! The framing of the sensor's HID reports: a packet header of an id byte
//! and a length byte, and the payload that follows it.
//!
//! A report the sensor sends starts with a byte that says what the packet
//! is, and a byte that says how long the packet is. That is the shape the
//! Goodix fingerprint protocol has in the parts that have been written
//! down, and the shape this crate parses.
//!
//! # The sizes are observed, not guaranteed
//!
//! [`ID_SIZE`] and [`LENGTH_SIZE`] are the sizes seen on the device in the
//! laptop: one byte for the id, one byte for the length. A firmware
//! revision of the sensor may frame its reports differently -- a wider
//! length field, or a length that counts the whole report rather than the
//! payload alone -- so the framing is a value a caller passes in
//! ([`Layout`]) rather than a constant the code assumes. The defaults
//! below are the observed ones.
//!
//! # What the vendor's own commands are not
//!
//! This module frames *packets*. It does not know what any packet means:
//! the opcodes the vendor defines are not published and live in
//! [`crate::vendor`], which is empty.

/// The size of the id byte at the head of a report.
pub const ID_SIZE: usize = 1;

/// The size of the length byte that follows it.
pub const LENGTH_SIZE: usize = 1;

/// The smallest a report can be with the default framing: the id and the
/// length, and nothing else.
pub const MIN_REPORT: usize = ID_SIZE + LENGTH_SIZE;

/// What the length byte of a report counts.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LengthCovers {
    /// The length byte counts the payload alone.
    #[default]
    Payload,
    /// The length byte counts the whole report, the header included.
    WholeReport,
}

/// How the sensor's reports are framed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Layout {
    /// How many bytes the length field takes. One is what the device in
    /// the laptop does; a wider field is what a different firmware
    /// revision could do.
    pub length_size: usize,
    /// What the length field counts.
    pub length_covers: LengthCovers,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            length_size: LENGTH_SIZE,
            length_covers: LengthCovers::Payload,
        }
    }
}

/// The framing observed on the device in the laptop: an id byte and a
/// length byte, and the length counts the payload.
pub const DEFAULT_LAYOUT: Layout = Layout {
    length_size: LENGTH_SIZE,
    length_covers: LengthCovers::Payload,
};

/// A packet: the id the sensor gave it, and the bytes that follow.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Packet<'a> {
    /// The packet's id: the first byte of the report.
    pub id: u8,
    /// The bytes after the header.
    pub payload: &'a [u8],
}

/// Why a report could not be read as a packet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Malformed {
    /// The report is shorter than its own header.
    TooShort {
        /// How many bytes the report had.
        got: usize,
        /// How many it needed.
        need: usize,
    },
    /// The length byte says the packet is longer than the report is.
    LengthMismatch {
        /// How many bytes the length byte declared.
        declared: usize,
        /// How many bytes were there to read.
        available: usize,
    },
    /// The packet's id is not one this driver knows.
    UnknownPacket {
        /// The id that was read.
        id: u8,
    },
    /// The framing itself is not one that can be read.
    BadLayout,
}

impl<'a> Packet<'a> {
    /// Read a report with the observed framing.
    ///
    /// # Errors
    ///
    /// [`Malformed::TooShort`] when the report is shorter than the
    /// header, and [`Malformed::LengthMismatch`] when the length byte
    /// declares more than the report holds.
    pub fn parse(bytes: &'a [u8]) -> Result<Packet<'a>, Malformed> {
        Packet::parse_with(bytes, DEFAULT_LAYOUT)
    }

    /// Read a report with a given framing.
    ///
    /// # Errors
    ///
    /// As [`Packet::parse`], and [`Malformed::BadLayout`] when the framing
    /// has no length field of a size that can be read.
    pub fn parse_with(bytes: &'a [u8], layout: Layout) -> Result<Packet<'a>, Malformed> {
        let need = header_size(layout);
        if bytes.len() < need {
            return Err(Malformed::TooShort {
                got: bytes.len(),
                need,
            });
        }
        let id = *bytes.first().ok_or(Malformed::TooShort { got: 0, need })?;
        let declared = declared_length(bytes, layout)?;
        let available = bytes.len().saturating_sub(need);
        if declared > available {
            return Err(Malformed::LengthMismatch {
                declared,
                available,
            });
        }
        let payload = bytes
            .get(need..need + declared)
            .ok_or(Malformed::LengthMismatch {
                declared,
                available,
            })?;
        Ok(Packet { id, payload })
    }

    /// Read a report, refusing an id that is not among the ones known.
    ///
    /// # Errors
    ///
    /// As [`Packet::parse_with`], and [`Malformed::UnknownPacket`] when
    /// the id read is not in `known`.
    pub fn parse_known(
        bytes: &'a [u8],
        layout: Layout,
        known: &[u8],
    ) -> Result<Packet<'a>, Malformed> {
        let packet = Packet::parse_with(bytes, layout)?;
        if known.contains(&packet.id) {
            Ok(packet)
        } else {
            Err(Malformed::UnknownPacket { id: packet.id })
        }
    }

    /// How many bytes the payload holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.payload.len()
    }

    /// Whether the payload is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.payload.is_empty()
    }
}

/// How many bytes the header of a report with this framing takes.
#[must_use]
pub fn header_size(layout: Layout) -> usize {
    ID_SIZE + layout.length_size
}

/// How many payload bytes the length field declares.
fn declared_length(bytes: &[u8], layout: Layout) -> Result<usize, Malformed> {
    if layout.length_size == 0 || layout.length_size > 4 {
        return Err(Malformed::BadLayout);
    }
    let mut value = 0_usize;
    for (index, byte) in bytes
        .get(ID_SIZE..ID_SIZE + layout.length_size)
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        value |= usize::from(*byte) << (8 * index);
    }
    Ok(match layout.length_covers {
        LengthCovers::Payload => value,
        LengthCovers::WholeReport => value.saturating_sub(header_size(layout)),
    })
}
