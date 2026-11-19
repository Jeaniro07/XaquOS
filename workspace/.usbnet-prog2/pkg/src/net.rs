//! The network side: where the frames off the wire go, and where the frames
//! to send come from.
//!
//! How a USB Ethernet interface reaches the kernel's network stack is not
//! decided yet: a net ring for a USB interface needs a kernel change that
//! has not been approved. So the seam is a trait with one implementation,
//! [`Loopback`], which hands a frame straight back: the USB half can then
//! be exercised in QEMU with `-device usb-net`, where every frame the
//! device sends comes back to it.
//!
//! Ponytail: the loopback is a stand-in, and the real transport is a net
//! ring over `xaqu-netserve`.
//! Upgrade path: implement [`Network`] over such a ring -- `deliver` hands
//! the frame to the ring, `take_outgoing` takes the next frame the stack
//! wants sent -- and keep [`Loopback`] as the fake the tests use.

use xaqu_usbnet::ecm::FRAME_HEADER_BYTES;

/// The most a frame may be: an Ethernet frame with its header, and the
/// padding a frame of few bytes carries.
pub(crate) const MAX_FRAME: usize = 2048;

/// The network side of a USB Ethernet interface.
///
/// The driver hands the frames it read to it, and takes from it the frames
/// to send: the seam the real transport will be behind.
pub(crate) trait Network {
    /// A frame came off the wire.
    fn deliver(&mut self, frame: &[u8]);

    /// The next frame the network side wants sent to the device: how many
    /// bytes are in `into`.
    fn take_outgoing(&mut self, into: &mut [u8]) -> Option<usize>;
}

/// The stand-in: a frame handed to it is echoed straight back.
///
/// It is one frame deep: a frame handed to it while another waits is
/// dropped, because the point is to exercise the USB half, not to be a
/// switch.
#[derive(Debug)]
pub(crate) struct Loopback {
    frame: [u8; MAX_FRAME],
    len: usize,
    pending: bool,
}

impl Loopback {
    /// A loopback with nothing to say yet.
    pub(crate) const fn new() -> Self {
        Loopback {
            frame: [0; MAX_FRAME],
            len: 0,
            pending: false,
        }
    }
}

impl Default for Loopback {
    fn default() -> Self {
        Self::new()
    }
}

impl Network for Loopback {
    fn deliver(&mut self, frame: &[u8]) {
        // A frame too short to be one is not a frame, and one too long for
        // the buffer is not one this driver can carry.
        if frame.len() < FRAME_HEADER_BYTES || frame.len() > MAX_FRAME {
            return;
        }
        let Some(slot) = self.frame.get_mut(..frame.len()) else {
            return;
        };
        slot.copy_from_slice(frame);
        self.len = frame.len();
        self.pending = true;
    }

    fn take_outgoing(&mut self, into: &mut [u8]) -> Option<usize> {
        if !self.pending {
            return None;
        }
        let len = self.len.min(into.len());
        let frame = self.frame.get(..len)?;
        let slot = into.get_mut(..len)?;
        slot.copy_from_slice(frame);
        self.pending = false;
        Some(len)
    }
}
