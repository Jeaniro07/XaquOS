//! The sensor's HID report descriptor, read for what a fingerprint reader
//! needs from it: the application collection it declares, the input reports
//! that carry the image, and the output report that carries a command.
//!
//! The HID report descriptor is parsed by the repository's own parser, in
//! `xaqu-usb-host`'s `report` module, so this crate does not keep a second
//! one. The grammar of a HID report descriptor is published (HID 1.11
//! §6.2.2) and parsing it twice would be two places to fix.
//!
//! What the descriptor does **not** say is the sensor's resolution: a
//! fingerprint reader's image size is not in the HID report descriptor,
//! and the vendor does not publish it either. [`SensorDescriptor`] therefore
//! reports the reports and their sizes, and the image's geometry is left to
//! [`crate::stream::ImageLayout`], which the caller fills in from a
//! capture or from a device-specific configuration.

use alloc::vec::Vec;

use xaqu_usb_host::report::{self, Descriptor, Direction};

/// The USB vendor id of the sensor this crate is written for.
pub const VENDOR_ID: u16 = 0x27c6;

/// The USB product id of the sensor this crate is written for.
pub const PRODUCT_ID: u16 = 0x5e0a;

/// The first of the HID usage pages the vendor defines for itself (HID 1.11
/// §4.3). A fingerprint reader declares its collection on such a page; the
/// parser reports which page it used rather than assuming one.
pub const PAGE_VENDOR_FIRST: u16 = 0xFF00;

/// A report the descriptor declares.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Report {
    /// The report's id, or 0 in a descriptor that declares no report ids.
    pub id: u8,
    /// How many bytes the report takes, not counting its id byte.
    pub bytes: usize,
    /// How many bits the report's fields take, not counting its id byte.
    pub bits: usize,
}

/// What the sensor's HID report descriptor says.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SensorDescriptor {
    /// The application collection's usage page and usage id, when the
    /// descriptor names one.
    pub application: Option<(u16, u16)>,
    /// The input reports, by ascending id.
    pub inputs: Vec<Report>,
    /// The output reports, by ascending id.
    pub outputs: Vec<Report>,
    /// Whether the descriptor gives its reports ids.
    pub numbered: bool,
}

/// Why a report descriptor could not be used.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Malformed {
    /// The HID report descriptor itself could not be parsed.
    Hid(report::Error),
    /// The descriptor declares no input report: nothing would ever arrive.
    NoInputReport,
    /// The descriptor declares no output report: no command could be sent.
    NoOutputReport,
}

impl SensorDescriptor {
    /// Parse the sensor's HID report descriptor.
    ///
    /// # Errors
    ///
    /// [`Malformed::Hid`] when the descriptor is not a HID report
    /// descriptor, [`Malformed::NoInputReport`] when it declares no input
    /// report, and [`Malformed::NoOutputReport`] when it declares no
    /// output report.
    pub fn parse(bytes: &[u8]) -> Result<SensorDescriptor, Malformed> {
        let parsed = Descriptor::parse(bytes).map_err(Malformed::Hid)?;
        let application = parsed
            .fields()
            .first()
            .map(|field| report::split(field.application));
        let inputs = reports(&parsed, Direction::Input);
        let outputs = reports(&parsed, Direction::Output);
        if inputs.is_empty() {
            return Err(Malformed::NoInputReport);
        }
        if outputs.is_empty() {
            return Err(Malformed::NoOutputReport);
        }
        Ok(SensorDescriptor {
            application,
            inputs,
            outputs,
            numbered: parsed.numbered,
        })
    }

    /// The input report with this id.
    #[must_use]
    pub fn input(&self, id: u8) -> Option<&Report> {
        self.inputs.iter().find(|entry| entry.id == id)
    }

    /// The output report with this id.
    #[must_use]
    pub fn output(&self, id: u8) -> Option<&Report> {
        self.outputs.iter().find(|entry| entry.id == id)
    }

    /// How many reports of this id an image of `image_bytes` takes.
    ///
    /// The image's size is not in the descriptor, so it is the caller's
    /// number; the answer is how many reports of that size it takes to
    /// carry it.
    #[must_use]
    pub fn packets_per_image(&self, image_bytes: usize, report_id: u8) -> Option<usize> {
        let report = self.input(report_id)?;
        if report.bytes == 0 || image_bytes == 0 {
            return None;
        }
        Some(image_bytes.div_ceil(report.bytes))
    }

    /// Whether the application collection is on a page the vendor defines.
    #[must_use]
    pub fn vendor_application(&self) -> bool {
        self.application
            .map(|(page, _)| page >= PAGE_VENDOR_FIRST)
            .unwrap_or(false)
    }
}

/// The reports the descriptor declares in one direction, by ascending id.
fn reports(parsed: &Descriptor, direction: Direction) -> Vec<Report> {
    let mut found: Vec<Report> = Vec::new();
    for field in parsed.fields() {
        if field.direction != direction {
            continue;
        }
        let bits = usize::from(field.offset) + usize::from(field.size) * usize::from(field.count);
        match found.iter_mut().find(|entry| entry.id == field.report) {
            Some(entry) => {
                if bits > entry.bits {
                    entry.bits = bits;
                    entry.bytes = bits.div_ceil(8);
                }
            }
            None => found.push(Report {
                id: field.report,
                bytes: bits.div_ceil(8),
                bits,
            }),
        }
    }
    found.sort_by_key(|entry| entry.id);
    found
}
