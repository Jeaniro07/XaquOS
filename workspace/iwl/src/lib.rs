//! Intel's AC 9560 Wi-Fi (the `iwlwifi` driver) as logic over the bytes it
//! speaks.
//!
//! The AC 9560 is a CNVi device: Intel's Wi-Fi MAC lives in the PCH and
//! only the radio sits on the PCI bus, behind a vendor-specific transport
//! that the Linux driver calls the PCIe transport. The driver is large; this
//! crate is the part of it that is a pure function of bytes and can be
//! tested on the host without the device:
//!
//! * [`firmware`] — the firmware file the device needs: its fixed header,
//!   its TLV directory and the record types in it.
//! * [`device`] — which PCI ids are which Intel Wi-Fi parts, what they are
//!   called, and which firmware file each of them loads.
//! * [`cmd`] — the host command header, the command id, the response status
//!   and the framing of a command into bytes.
//!
//! What is *not* here, and what the next crates have to be: the EEPROM
//! (the NVM) parsing, the RF calibration tables, the 802.11 MAC itself, the
//! scan and association state machines, and WPA2. Nothing in this crate
//! associates with a network, and nothing in this crate is a key exchange.
//! `docs/IWLIFI.md` says what the whole path is.
//!
//! The formats are the Linux kernel's, read from the `iwlwifi` tree of
//! Linux v6.6 (`drivers/net/wireless/intel/iwlwifi/`): `fw/file.h` for the
//! firmware file, `fw/api/cmdhdr.h` and `fw/api/commands.h` for the
//! command framing, `pcie/drv.c` and `cfg/9000.c` for the device table.
//! No code was taken from the kernel; only the formats were read.
//!
//! ```text
//! firmware.rs   the .ucode file: header, TLV directory, record types
//! device.rs     the PCI ids, the names, the firmware file names
//! cmd.rs        the host command header, the command id, the responses
//! ```

#![no_std]
#![forbid(unsafe_code)]
#![forbid(missing_docs)]

pub mod cmd;
pub mod device;
pub mod firmware;

#[cfg(test)]
mod tests;
