//! The message `devmgr` starts the driver with.
//!
//! The driver is a ring-3 program with no way to walk the PCI configuration
//! space itself, so `devmgr` tells it what the kernel's enumeration found:
//! which device it is, and which of the device's base address registers
//! holds the controller's register block. The driver asks the kernel for
//! that aperture and maps it, as the NVMe and AHCI drivers do.

use xaqu_native_abi::rights::Rights;

/// The message's type.
pub const START: u32 = 1;
/// The bytes of the type and the length every message starts with.
pub const HEADER_BYTES: usize = 8;
/// The bytes of START.
pub const START_BYTES: usize = 24;

/// Exactly the rights a driver holds its device with: `MANAGE`, for the
/// aperture, the mapping and the interrupt, and the `TRANSFER` it arrived
/// with.
pub const DEVICE_RIGHTS: Rights = Rights(Rights::TRANSFER.0 | Rights::MANAGE.0);

/// Exactly the rights a driver holds its end of the sound control channel
/// with: it sends and receives on it, waits on it and was handed it.
pub const CONTROL_RIGHTS: Rights =
    Rights(Rights::TRANSFER.0 | Rights::READ.0 | Rights::WRITE.0 | Rights::WAIT.0);

/// START's handles, in order, with exactly the rights each must carry.
pub const START_RIGHTS: [Rights; 2] = [DEVICE_RIGHTS, CONTROL_RIGHTS];

/// START: the device the driver is to drive.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Start {
    /// The protocol version this crate speaks.
    pub version: u16,
    /// The device's place among the devices the kernel enumerated.
    pub location: u32,
    /// The device's PCI identifier, as the enumeration gave it.
    pub pci_device_id: u16,
    /// Which of the device's base address registers holds the register block.
    pub bar: u8,
}

impl Start {
    /// START's bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; START_BYTES] {
        let mut out = [0_u8; START_BYTES];
        put32(&mut out, 0, START);
        put32(&mut out, 4, START_BYTES as u32);
        put16(&mut out, 8, self.version);
        put32(&mut out, 12, self.location);
        put16(&mut out, 16, self.pci_device_id);
        out[18] = self.bar;
        out
    }

    /// The START the bytes carry.
    ///
    /// # Errors
    ///
    /// `None` if the bytes are not one well-formed START.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != START_BYTES {
            return None;
        }
        if get32(bytes, 0)? != START || get32(bytes, 4)? as usize != START_BYTES {
            return None;
        }
        Some(Self {
            version: get16(bytes, 8)?,
            location: get32(bytes, 12)?,
            pci_device_id: get16(bytes, 16)?,
            bar: *bytes.get(18)?,
        })
    }
}

fn get16(bytes: &[u8], at: usize) -> Option<u16> {
    let field = bytes.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(field.try_into().ok()?))
}

fn get32(bytes: &[u8], at: usize) -> Option<u32> {
    let field = bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(field.try_into().ok()?))
}

fn put16(out: &mut [u8], at: usize, value: u16) {
    if let Some(slot) = out.get_mut(at..at + 2) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

fn put32(out: &mut [u8], at: usize, value: u32) {
    if let Some(slot) = out.get_mut(at..at + 4) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}
