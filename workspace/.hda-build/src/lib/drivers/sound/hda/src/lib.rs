//! The Intel HD Audio controller, as logic over the registers it is handed.
//!
//! An HD Audio controller is a host controller with a codec behind it. The
//! host's register block holds the CORB (the command ring the verbs go out
//! on), the RIRB (the ring the codec's answers come back on), and one block
//! of registers per stream; the codec is a separate device, reachable only
//! through verbs: a four-byte command naming a codec, a node and a
//! parameter, with the answer coming back in the RIRB.
//!
//! Everything here is a pure function of the bytes in and out: the driver's
//! own program maps the register block, fills the rings and reads the
//! answers. That is what makes it testable on the host, where a fake register
//! block is a byte array and a fake codec is a table of answers.
//!
//! The register offsets, the verbs and the widget and pin bits are the Intel
//! High Definition Audio specification's (revision 1.0a), as the Linux
//! driver `sound/pci/hda` and `sound/hda` read them; `docs/HDA.md` §2 says
//! which is which.
//!
//! ```text
//! controller.rs  the register block, and the reset and the rings over it
//! corb.rs        the verbs, and the CORB and RIRB as rings
//! codec.rs       what a codec says about its nodes
//! path.rs        which path a device's sound takes through the codec
//! bdl.rs         the buffer descriptor list a stream plays from
//! control.rs     the message `devmgr` starts the driver with
//! ```

#![no_std]
#![cfg_attr(not(test), forbid(unsafe_code))]
#![cfg_attr(test, deny(unsafe_code))]

pub mod bdl;
pub mod codec;
pub mod control;
pub mod controller;
pub mod corb;
pub mod path;

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests;
