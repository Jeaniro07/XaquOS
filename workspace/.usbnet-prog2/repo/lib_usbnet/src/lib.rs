//! USB Ethernet dongle protocols as logic over the bytes they carry.
//!
//! A USB Ethernet dongle is a small controller with a firmware that speaks
//! one of three protocols, and the protocol is the whole of what the driver
//! has to know: the control requests that bring the device up, and the
//! shape of what comes back on the bulk endpoints.
//!
//! This crate is that knowledge and nothing else. It does not touch the
//! bus, does not open an endpoint and does not move a byte that is not
//! already in a buffer: every function here is a pure function of the bytes
//! it is given and the bytes it returns. That is what makes it testable on
//! the host, where a fake dongle is a table of answers and a buffer.
//!
//! * [`cdc`]: the Communications Device Class, the requests and the
//!   notifications every dongle answers before it carries a frame;
//! * [`ecm`]: the Ethernet Control Model, one Ethernet frame per bulk
//!   transfer;
//! * [`ncm`]: the Network Control Model, Ethernet frames batched into
//!   transfer blocks;
//! * [`rndis`]: the Remote NDIS protocol, Microsoft's framing of the same
//!   idea, carried in CDC's encapsulated commands.
//!
//! The request codes, the structure layouts and the bit meanings are the
//! Linux kernel's own, as `include/uapi/linux/usb/cdc.h`,
//! `include/linux/usb/rndis_host.h`, `drivers/net/usb/cdc_ether.c`,
//! `drivers/net/usb/cdc_ncm.c` and `drivers/net/usb/rndis_host.c` read them.
//!
//! ```text
//! cdc.rs    the class requests, the notifications, the line coding
//! ecm.rs    the packet filter setup, and the frame in and out
//! ncm.rs    the transfer blocks, the datagram pointers, the block setup
//! rndis.rs  the messages, the identifiers, the states the device is in
//! ```

#![no_std]
#![forbid(unsafe_code)]
#![forbid(missing_docs)]

mod bytes;

pub mod cdc;
pub mod ecm;
pub mod ncm;
pub mod rndis;

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests;
