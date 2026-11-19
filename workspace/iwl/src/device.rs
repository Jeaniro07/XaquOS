//! The Intel Wi-Fi devices this crate knows: their PCI ids, their names and
//! the firmware file they need.
//!
//! The AC 9560 is a CNVi device: the MAC is in the PCH and only a small
//! companion ("CRF", the radio) is on the PCI bus, which is why the device
//! is behind a vendor-specific transport rather than the plain one. Intel
//! ships the AC 9560 and its relatives under two names in the Linux
//! driver, and this crate keeps both:
//!
//! * the **9000** family (`IWL_DEVICE_FAMILY_9000`), whose firmware is
//!   named `iwlwifi-9000-pu-b0-jf-b0-<api>.ucode`;
//! * the **Qu** family (CNVi parts such as the AX201), whose firmware is
//!   named `iwlwifi-Qu-b0-<rf>-b0-<api>.ucode`.
//!
//! The table is the kernel's own. The device ids and which transport
//! configuration each one selects are read from
//! `drivers/net/wireless/intel/iwlwifi/pcie/drv.c` (Linux v6.6), the
//! `IWL_PCI_DEVICE` entries of `iwl_pci_devices`; the family, the API
//! version range and the firmware prefix are read from
//! `drivers/net/wireless/intel/iwlwifi/cfg/9000.c` and `cfg/ax210.c`
//! (`IWL9000_UCODE_API_MIN`, `IWL9000_UCODE_API_MAX`, `IWL9000_FW_PRE`);
//! the names are the `iwl9560_name` and `iwl9462_name` strings of the same
//! files.
//!
//! The firmware file name is built the way the driver builds it:
//! `iwl_drv_get_fwname_pre` in `drivers/net/wireless/intel/iwlwifi/iwl-drv.c`
//! makes `iwlwifi-<mac>-<mac step>0-<rf><cdb>-<rf step>0`, and
//! `iwl_request_firmware` in the same file appends `-<api>.ucode`.

use core::fmt;

/// Intel's PCI vendor id.
pub const INTEL_VENDOR_ID: u16 = 0x8086;

/// Intel's USB vendor id, the one the companion Bluetooth part carries.
pub const INTEL_BLUETOOTH_VENDOR_ID: u16 = 0x8087;

/// The USB product id of the Bluetooth companion of the AC 9560.
pub const BLUETOOTH_AC9560_PRODUCT_ID: u16 = 0x0aaa;

/// The firmware prefix of the 9000 family.
///
/// Upstream: `IWL9000_FW_PRE`, `cfg/9000.c`.
pub const FW_PRE_9000_PU: &str = "iwlwifi-9000-pu-b0-jf-b0";

/// The firmware prefix of the Qu family, B-step, with the "HR" radio.
///
/// The file names `iwlwifi-Qu-b0-hr-b0-<api>.ucode` are the ones the
/// `firmware-iwlwifi` package ships.
pub const FW_PRE_QU_B0_HR_B0: &str = "iwlwifi-Qu-b0-hr-b0";

/// The firmware prefix of the Qu family, B-step, with the "JF" radio.
pub const FW_PRE_QU_B0_JF_B0: &str = "iwlwifi-Qu-b0-jf-b0";

/// The firmware API versions the 9000 family ships, as the `firmware-iwlwifi`
/// package of Debian bookworm lists them.
pub const PUBLISHED_9000_PU_APIS: &[u8] = &[34, 38, 46];

/// The firmware API versions the Qu family ships, as the `firmware-iwlwifi`
/// package of Debian bookworm lists them.
pub const PUBLISHED_QU_APIS: &[u8] = &[50, 59, 72];

/// The longest firmware file name this crate builds, plus room for the
/// terminating zero.
pub const FIRMWARE_NAME_MAX: usize = 48;

/// Which Intel Wi-Fi family a device belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceFamily {
    /// The 9000 series: 9461, 9462, 9560 and the 9260.
    Iwl9000,
    /// The CNVi "Qu" parts: AX201 and its relatives.
    Qu,
}

impl DeviceFamily {
    /// The name of the family, as the kernel writes it.
    pub fn name(self) -> &'static str {
        match self {
            DeviceFamily::Iwl9000 => "Iwl9000",
            DeviceFamily::Qu => "Qu",
        }
    }
}

/// One device of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Device {
    /// The PCI device id.
    pub pci_device_id: u16,
    /// The name the Linux driver gives the device.
    pub name: &'static str,
    /// The family the device belongs to.
    pub family: DeviceFamily,
    /// The prefix of the firmware file the device needs.
    pub firmware_prefix: &'static str,
    /// The lowest firmware API version the driver accepts.
    pub ucode_api_min: u8,
    /// The highest firmware API version the driver accepts.
    pub ucode_api_max: u8,
}

/// The devices of the table.
///
/// The ids are the `IWL_PCI_DEVICE` entries of `iwl_pci_devices` in
/// `pcie/drv.c`; `0x9DF0`, `0xA370`, `0x30DC` and `0x31DC` are the AC 9560
/// and its variants, and `0xA0F0` is the CNVi part the Linux driver binds
/// to the AX201 configuration.
const DEVICES: &[Device] = &[
    Device {
        pci_device_id: 0x9df0,
        name: "Intel(R) Wireless-AC 9560",
        family: DeviceFamily::Iwl9000,
        firmware_prefix: FW_PRE_9000_PU,
        ucode_api_min: 30,
        ucode_api_max: 46,
    },
    Device {
        pci_device_id: 0xa370,
        name: "Intel(R) Wireless-AC 9560",
        family: DeviceFamily::Iwl9000,
        firmware_prefix: FW_PRE_9000_PU,
        ucode_api_min: 30,
        ucode_api_max: 46,
    },
    Device {
        pci_device_id: 0x30dc,
        name: "Intel(R) Wireless-AC 9560",
        family: DeviceFamily::Iwl9000,
        firmware_prefix: FW_PRE_9000_PU,
        ucode_api_min: 30,
        ucode_api_max: 46,
    },
    Device {
        pci_device_id: 0x31dc,
        name: "Intel(R) Wireless-AC 9560",
        family: DeviceFamily::Iwl9000,
        firmware_prefix: FW_PRE_9000_PU,
        ucode_api_min: 30,
        ucode_api_max: 46,
    },
    Device {
        pci_device_id: 0xa0f0,
        name: "Wi-Fi 6 AX201",
        family: DeviceFamily::Qu,
        firmware_prefix: FW_PRE_QU_B0_HR_B0,
        ucode_api_min: 59,
        ucode_api_max: 83,
    },
];

/// The device with the given PCI device id.
pub fn lookup(pci_device_id: u16) -> Option<&'static Device> {
    DEVICES.iter().find(|device| device.pci_device_id == pci_device_id)
}

/// The device with the given PCI vendor and device id.
///
/// Only Intel's vendor id is looked up; the Bluetooth companion is a USB
/// device and is not a PCI function.
pub fn lookup_by_pci(vendor_id: u16, pci_device_id: u16) -> Option<&'static Device> {
    if vendor_id != INTEL_VENDOR_ID {
        return None;
    }
    lookup(pci_device_id)
}

/// The firmware file name for a device and an API version.
///
/// The name is `<prefix>-<api>.ucode`, the way `iwl_request_firmware` in
/// `iwl-drv.c` builds it.
pub fn firmware_file_name(prefix: &str, api: u8) -> FirmwareName {
    let mut name = FirmwareName::new();
    push_bytes(&mut name, prefix.as_bytes());
    push_bytes(&mut name, b"-");
    push_bytes(&mut name, decimal(api).as_slice());
    push_bytes(&mut name, b".ucode");
    name
}

/// The decimal form of `value`, without allocating.
fn decimal(value: u8) -> [u8; 3] {
    let mut digits = [0_u8; 3];
    if value == 0 {
        digits[0] = b'0';
        return digits;
    }
    let mut count = 0;
    let mut rest = value;
    while rest > 0 && count < 3 {
        digits[count] = b'0' + rest % 10;
        rest /= 10;
        count += 1;
    }
    if let Some(head) = digits.get_mut(..count) {
        head.reverse();
    }
    digits
}

/// Appends bytes to a name, stopping at its capacity.
fn push_bytes(name: &mut FirmwareName, bytes: &[u8]) {
    for byte in bytes {
        if name.len + 1 >= FIRMWARE_NAME_MAX {
            return;
        }
        if let Some(slot) = name.bytes.get_mut(name.len) {
            *slot = *byte;
        }
        name.len += 1;
    }
}

/// A firmware file name held in a fixed buffer.
#[derive(Debug, Clone)]
pub struct FirmwareName {
    bytes: [u8; FIRMWARE_NAME_MAX],
    len: usize,
}

impl FirmwareName {
    /// An empty name.
    pub fn new() -> FirmwareName {
        FirmwareName {
            bytes: [0; FIRMWARE_NAME_MAX],
            len: 0,
        }
    }

    /// The name as a string, empty if the bytes are not text.
    pub fn as_str(&self) -> &str {
        let bytes = self.bytes.get(..self.len).unwrap_or(&[]);
        core::str::from_utf8(bytes).unwrap_or("")
    }

    /// The name as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or(&[])
    }
}

impl Default for FirmwareName {
    fn default() -> Self {
        FirmwareName::new()
    }
}

impl fmt::Display for FirmwareName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
