//! A Goodix fingerprint reader on USB: what of it can be verified, and where
//! its vendor protocol is still unknown.
//!
//! The sensor in the laptop this tree is written for is a Goodix
//! fingerprint reader on the bus, USB `27c6:5e0a`. What the USB and HID
//! specifications say about it is what this crate implements: the HID
//! report descriptor the device publishes, the reports it sends and
//! receives, the framing of the packets inside those reports, and the
//! reassembly of an image out of a sequence of packets.
//!
//! What the *vendor's* protocol says -- the opcodes that ask whether a
//! finger is on the sensor, that start and stop a capture, that enroll a
//! finger, that match one, that store and load a template -- is **not
//! published**. There is no specification to read, so this crate carries
//! an *empty* table of those opcodes ([`vendor`]) and every attempt to
//! build a request out of it fails with [`vendor::Error::UnknownOpcode`]
//! until the opcodes are filled in from a capture of the real device.
//! That is the point of the table: it makes the gap explicit and
//! testable instead of hiding it behind a plausible-looking constant.
//!
//! **Nothing here claims that enrollment works.** It cannot: the command
//! that enrolls a finger is one of the unknowns. See `docs/FINGERPRINT.md`.
//!
//! # The modules
//!
//! * [`descriptor`]: the HID report descriptor, read for what a
//!   fingerprint reader needs;
//! * [`packet`]: the framing of a report into a packet;
//! * [`stream`]: the reassembly of an image out of a sequence of packets;
//! * [`vendor`]: the empty table of vendor opcodes, and its plumbing.
//!
//! # What is not here
//!
//! No bus, no USB transfer, no timing, no template store: this crate is
//! pure logic over bytes, testable on the host with hand-written
//! fixtures. The ring-3 program that would drive the device is a separate
//! program (`docs/DRIVER-MODEL.md`).

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod descriptor;
pub mod packet;
pub mod stream;
pub mod vendor;

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests;
