//! The reassembly of an image out of the sensor's packets.
//!
//! The sensor delivers an image as a sequence of packets: one that starts
//! the image, a run of ones that carry its pieces, and one that ends it.
//! [`Stream`] is the state machine that follows such a sequence and hands
//! back the image when the end packet arrives.
//!
//! # Which id means what is a parameter
//!
//! The vendor does not publish which packet id starts an image, which
//! carries its data and which ends it, so [`StreamLayout`] takes the
//! three ids as values. There is no default triple, because a default
//! triple would be a claim about the device that nobody has checked.
//!
//! # The geometry is a parameter too
//!
//! The image's width, height and resolution are not in the HID report
//! descriptor and not published by the vendor, so [`ImageLayout`] carries
//! them as values and its default says "unknown" (zero) rather than a
//! made-up number. A caller that has read a capture, or a
//! device-specific configuration, fills them in; until then a frame's
//! [`Frame::is_complete`] says whether the bytes and the geometry agree.

use alloc::vec::Vec;

use crate::packet::Packet;

/// The geometry of an image: its size, and the sensor's resolution.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ImageLayout {
    /// The image's width in pixels.
    pub width: u16,
    /// The image's height in pixels.
    pub height: u16,
    /// The sensor's resolution in dots per inch, or 0 when it is unknown.
    pub dpi: u16,
}

impl ImageLayout {
    /// The layout of an image whose geometry is not known.
    ///
    /// The default, and what a caller keeps until it has read a capture or
    /// a device-specific configuration: a width and a height of zero say
    /// "not known", which is true, rather than a resolution nobody has
    /// measured.
    #[must_use]
    pub const fn unknown() -> ImageLayout {
        ImageLayout {
            width: 0,
            height: 0,
            dpi: 0,
        }
    }

    /// How many pixels an image of this layout has.
    #[must_use]
    pub fn pixels(&self) -> usize {
        usize::from(self.width) * usize::from(self.height)
    }

    /// How many bytes an image of this layout takes, one byte per pixel.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.pixels()
    }
}

/// Which packet ids mean what, and how big the image is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StreamLayout {
    /// The id of the packet that starts an image.
    pub start_id: u8,
    /// The id of the packet that carries a piece of the image.
    pub data_id: u8,
    /// The id of the packet that ends an image.
    pub end_id: u8,
    /// The image's geometry.
    pub image: ImageLayout,
}

/// An image, reassembled.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Frame {
    /// The image's width in pixels.
    pub width: u16,
    /// The image's height in pixels.
    pub height: u16,
    /// The sensor's resolution in dots per inch, or 0 when it is unknown.
    pub dpi: u16,
    /// The pixels, one byte each, in the order they were carried.
    pub data: Vec<u8>,
}

impl Frame {
    /// How many bytes a whole image of this size takes.
    #[must_use]
    pub fn expected_bytes(&self) -> usize {
        usize::from(self.width) * usize::from(self.height)
    }

    /// Whether the frame holds a whole image: its bytes and its geometry
    /// agree.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.data.len() == self.expected_bytes()
    }
}

/// The state machine that reassembles an image.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Stream {
    layout: StreamLayout,
    buffer: Vec<u8>,
    started: bool,
    packets: usize,
}

impl Stream {
    /// A stream that follows `layout`.
    #[must_use]
    pub fn new(layout: StreamLayout) -> Stream {
        Stream {
            layout,
            buffer: Vec::new(),
            started: false,
            packets: 0,
        }
    }

    /// The layout this stream follows.
    #[must_use]
    pub const fn layout(&self) -> &StreamLayout {
        &self.layout
    }

    /// How many bytes of the image in hand have been collected.
    #[must_use]
    pub fn buffered(&self) -> usize {
        self.buffer.len()
    }

    /// How many data packets the image in hand has had.
    #[must_use]
    pub fn packets(&self) -> usize {
        self.packets
    }

    /// Forget the image in hand.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.started = false;
        self.packets = 0;
    }

    /// Give the stream a packet; the image comes back when the packet
    /// that ends it arrives.
    ///
    /// A packet whose id is none of the three the layout names is ignored:
    /// the device sends reports of its own on the same endpoint, and this
    /// stream has nothing to do with them.
    pub fn push(&mut self, packet: &Packet<'_>) -> Option<Frame> {
        if packet.id == self.layout.start_id {
            self.reset();
            self.started = true;
            return None;
        }
        if packet.id == self.layout.data_id {
            if !self.started {
                return None;
            }
            self.buffer.extend_from_slice(packet.payload);
            self.packets += 1;
            return None;
        }
        if packet.id == self.layout.end_id {
            if !self.started || self.buffer.is_empty() {
                return None;
            }
            let data = core::mem::take(&mut self.buffer);
            self.started = false;
            self.packets = 0;
            return Some(Frame {
                width: self.layout.image.width,
                height: self.layout.image.height,
                dpi: self.layout.image.dpi,
                data,
            });
        }
        None
    }
}
