//! The HID descriptor a device on an I2C bus keeps.
//!
//! Thirty bytes, little-endian, as Linux's `drivers/hid/i2c-hid.c` reads it:
//! the length of the report descriptor and the register it lies at, the
//! input register and the most bytes an input report may be, the output
//! register and its length, the command and data registers, and the
//! device's own numbers.
//!
//! The register numbers are the point of the descriptor. A driver that
//! assumes the default registers reads the wrong bytes on any device that
//! was told to move them, so the descriptor is read first, always.

/// The length of a HID descriptor.
pub const DESCRIPTOR_BYTES: usize = 30;

/// Why a descriptor could not be read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Malformed {
    /// The bytes were fewer than [`DESCRIPTOR_BYTES`].
    Short,
    /// A register number was zero: a device that says it keeps nothing at a
    /// register it names is a device whose descriptor cannot be used.
    ZeroRegister,
    /// A length was longer than the register's own allowance.
    LengthPastRegister,
}

/// The descriptor, as it was read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Descriptor {
    /// The length of the descriptor itself.
    pub desc_length: u16,
    /// The version of the HID over I2C protocol the device speaks.
    pub bcd_version: u16,
    /// How long the report descriptor is.
    pub report_desc_length: u16,
    /// The register the report descriptor is read from.
    pub report_desc_register: u16,
    /// The register an input report is read from.
    pub input_register: u16,
    /// The most bytes an input report may be.
    pub max_input_length: u16,
    /// The register an output report is written to.
    pub output_register: u16,
    /// The most bytes an output report may be.
    pub max_output_length: u16,
    /// The register a command is written to.
    pub command_register: u16,
    /// The register a command's data is written to.
    pub data_register: u16,
    /// The device's vendor.
    pub vendor_id: u16,
    /// The device's product.
    pub product_id: u16,
    /// The device's version.
    pub version_id: u16,
}

impl Descriptor {
    /// Read the descriptor at the start of `bytes`.
    ///
    /// # Errors
    ///
    /// [`Malformed::Short`] when the bytes are fewer than
    /// [`DESCRIPTOR_BYTES`], [`Malformed::ZeroRegister`] when a register the
    /// device must have is zero, and [`Malformed::LengthPastRegister`] when
    /// a length is longer than the register it belongs to.
    pub fn parse(bytes: &[u8]) -> Result<Descriptor, Malformed> {
        if bytes.len() < DESCRIPTOR_BYTES {
            return Err(Malformed::Short);
        }
        let mut words = [0_u16; 13];
        for (slot, at) in words.iter_mut().zip((0..13).map(|word| word * 2)) {
            let pair = bytes.get(at..at + 2).ok_or(Malformed::Short)?;
            let low = *pair.first().ok_or(Malformed::Short)?;
            let high = *pair.get(1).ok_or(Malformed::Short)?;
            *slot = u16::from_le_bytes([low, high]);
        }
        let descriptor = Descriptor {
            desc_length: words[0],
            bcd_version: words[1],
            report_desc_length: words[2],
            report_desc_register: words[3],
            input_register: words[4],
            max_input_length: words[5],
            output_register: words[6],
            max_output_length: words[7],
            command_register: words[8],
            data_register: words[9],
            vendor_id: words[10],
            product_id: words[11],
            version_id: words[12],
        };
        descriptor.check()?;
        Ok(descriptor)
    }

    /// The descriptor's own bytes, as a device keeps them.
    #[must_use]
    pub fn to_bytes(self) -> [u8; DESCRIPTOR_BYTES] {
        let words = [
            self.desc_length,
            self.bcd_version,
            self.report_desc_length,
            self.report_desc_register,
            self.input_register,
            self.max_input_length,
            self.output_register,
            self.max_output_length,
            self.command_register,
            self.data_register,
            self.vendor_id,
            self.product_id,
            self.version_id,
        ];
        let mut bytes = [0_u8; DESCRIPTOR_BYTES];
        for (at, word) in words.iter().enumerate() {
            let pair = word.to_le_bytes();
            if let Some(slot) = bytes.get_mut(at * 2..at * 2 + 2) {
                slot.copy_from_slice(&pair);
            }
        }
        bytes
    }

    /// Whether the descriptor is usable.
    fn check(&self) -> Result<(), Malformed> {
        if self.report_desc_register == 0
            || self.input_register == 0
            || self.command_register == 0
            || self.data_register == 0
        {
            return Err(Malformed::ZeroRegister);
        }
        if self.report_desc_length == 0 || self.max_input_length == 0 {
            return Err(Malformed::LengthPastRegister);
        }
        Ok(())
    }
}
