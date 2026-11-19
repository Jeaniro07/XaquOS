//! The tests of the HID over I2C protocol.

use super::{Command, Descriptor, Malformed, Opcode, ReportType, DESCRIPTOR_BYTES};

/// A descriptor as a device keeps it: the default registers Linux's
/// `drivers/hid/i2c-hid.c` expects a device to use.
fn default_descriptor() -> [u8; DESCRIPTOR_BYTES] {
    let words = [
        0x0028_u16, // wHIDDescLength
        0x0001,     // bcdVersion
        0x0022,     // wReportDescLength
        0x0002,     // wReportDescRegister
        0x0003,     // wInputRegister
        0x0040,     // wMaxInputLength
        0x0004,     // wOutputRegister
        0x0020,     // wMaxOutputLength
        0x0005,     // wCommandRegister
        0x0006,     // wDataRegister
        0x1234,     // wVendorID
        0x5678,     // wProductID
        0x0001,     // wVersionID
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

#[test]
fn the_descriptor_is_read_as_linux_reads_it() {
    let bytes = default_descriptor();
    let descriptor = Descriptor::parse(&bytes).expect("the descriptor is a good one");
    assert_eq!(descriptor.bcd_version, 0x0001);
    assert_eq!(descriptor.report_desc_length, 0x0022);
    assert_eq!(descriptor.report_desc_register, 0x0002);
    assert_eq!(descriptor.input_register, 0x0003);
    assert_eq!(descriptor.max_input_length, 0x0040);
    assert_eq!(descriptor.output_register, 0x0004);
    assert_eq!(descriptor.max_output_length, 0x0020);
    assert_eq!(descriptor.command_register, 0x0005);
    assert_eq!(descriptor.data_register, 0x0006);
    assert_eq!(descriptor.vendor_id, 0x1234);
    assert_eq!(descriptor.product_id, 0x5678);
    assert_eq!(descriptor.version_id, 0x0001);
}

#[test]
fn a_short_descriptor_is_refused() {
    let bytes = default_descriptor();
    let short = bytes.get(..DESCRIPTOR_BYTES - 1).unwrap_or_default();
    assert_eq!(Descriptor::parse(short), Err(Malformed::Short));
    assert_eq!(Descriptor::parse(&[]), Err(Malformed::Short));
}

#[test]
fn a_descriptor_without_registers_is_refused() {
    let mut bytes = default_descriptor();
    bytes[16] = 0x00; // wCommandRegister, low byte
    bytes[17] = 0x00;
    assert_eq!(Descriptor::parse(&bytes), Err(Malformed::ZeroRegister));
}

#[test]
fn a_descriptor_without_a_report_is_refused() {
    let mut bytes = default_descriptor();
    bytes[4] = 0x00; // wReportDescLength, low byte
    bytes[5] = 0x00;
    assert_eq!(Descriptor::parse(&bytes), Err(Malformed::LengthPastRegister));
}

#[test]
fn a_descriptor_survives_the_round_trip() {
    let bytes = default_descriptor();
    let descriptor = Descriptor::parse(&bytes).expect("the descriptor is a good one");
    assert_eq!(descriptor.to_bytes(), bytes);
}

#[test]
fn the_opcodes_are_the_ones_linux_names() {
    assert_eq!(Opcode::Reset.code(), 0x01);
    assert_eq!(Opcode::GetReport.code(), 0x02);
    assert_eq!(Opcode::SetReport.code(), 0x03);
    assert_eq!(Opcode::GetIdle.code(), 0x04);
    assert_eq!(Opcode::SetIdle.code(), 0x05);
    assert_eq!(Opcode::GetProtocol.code(), 0x06);
    assert_eq!(Opcode::SetProtocol.code(), 0x07);
    assert_eq!(Opcode::SetPower.code(), 0x08);
}

#[test]
fn an_opcode_survives_the_round_trip() {
    for opcode in [
        Opcode::Reset,
        Opcode::GetReport,
        Opcode::SetReport,
        Opcode::GetIdle,
        Opcode::SetIdle,
        Opcode::GetProtocol,
        Opcode::SetProtocol,
        Opcode::SetPower,
    ] {
        assert_eq!(Opcode::from_code(opcode.code()), Some(opcode));
    }
    assert_eq!(Opcode::from_code(0x00), None);
    assert_eq!(Opcode::from_code(0x09), None);
    assert_eq!(Opcode::from_code(0xFF), None);
}

#[test]
fn the_report_types_are_the_ones_the_specification_names() {
    assert_eq!(ReportType::Input.code(), 1);
    assert_eq!(ReportType::Output.code(), 2);
    assert_eq!(ReportType::Feature.code(), 3);
}

#[test]
fn a_reset_is_four_bytes_and_names_no_report() {
    let command = Command::Reset;
    let mut out = [0_u8; 8];
    let at = command.encode(0x0005, 0x0006, &mut out);
    assert_eq!(at, 4);
    assert_eq!(out.get(..4), Some([0x05, 0x00, 0x00, 0x01].as_slice()));
    assert_eq!(command.kind(), None);
}

#[test]
fn a_get_report_names_the_report_in_its_data() {
    let command = Command::GetReport {
        kind: ReportType::Feature,
        id: 0x07,
    };
    let mut out = [0_u8; 8];
    let at = command.encode(0x0005, 0x0006, &mut out);
    assert_eq!(at, 7);
    assert_eq!(
        out.get(..7),
        Some([0x05, 0x00, 0x03, 0x02, 0x07, 0x06, 0x00].as_slice())
    );
}

#[test]
fn a_set_report_carries_its_bytes() {
    let command = Command::SetReport {
        kind: ReportType::Output,
        id: 0x01,
        data: &[0xAA, 0xBB],
    };
    let mut out = [0_u8; 16];
    let at = command.encode(0x0005, 0x0006, &mut out);
    assert_eq!(at, 9);
    assert_eq!(
        out.get(..9),
        Some([0x05, 0x00, 0x02, 0x03, 0x01, 0x06, 0x00, 0xAA, 0xBB].as_slice())
    );
}

#[test]
fn an_idle_time_is_two_bytes_little_endian() {
    let command = Command::SetIdle { milliseconds: 0x0123 };
    let mut out = [0_u8; 8];
    let at = command.encode(0x0005, 0x0006, &mut out);
    assert_eq!(at, 6);
    assert_eq!(out.get(..6), Some([0x05, 0x00, 0x00, 0x05, 0x23, 0x01].as_slice()));
}

#[test]
fn a_protocol_and_a_power_are_one_byte_each() {
    let mut out = [0_u8; 8];
    let protocol = Command::SetProtocol { protocol: 1 };
    assert_eq!(protocol.encode(0x0005, 0x0006, &mut out), 5);
    assert_eq!(out.get(..5), Some([0x05, 0x00, 0x00, 0x07, 0x01].as_slice()));
    let power = Command::SetPower { state: 0 };
    assert_eq!(power.encode(0x0005, 0x0006, &mut out), 5);
    assert_eq!(out.get(..5), Some([0x05, 0x00, 0x00, 0x08, 0x00].as_slice()));
}

#[test]
fn the_length_a_command_announces_is_the_length_it_takes() {
    let commands = [
        Command::Reset,
        Command::GetReport {
            kind: ReportType::Input,
            id: 1,
        },
        Command::SetReport {
            kind: ReportType::Output,
            id: 1,
            data: &[1, 2, 3],
        },
        Command::GetIdle,
        Command::SetIdle { milliseconds: 5 },
        Command::GetProtocol,
        Command::SetProtocol { protocol: 1 },
        Command::SetPower { state: 1 },
    ];
    for command in commands {
        let mut out = [0_u8; 32];
        let at = command.encode(0x0005, 0x0006, &mut out);
        assert_eq!(at, command.length(), "the length it announced");
    }
}

#[test]
fn a_buffer_that_is_too_short_takes_what_fits() {
    let command = Command::SetReport {
        kind: ReportType::Output,
        id: 1,
        data: &[1, 2, 3, 4],
    };
    let mut out = [0_u8; 8];
    let at = command.encode(0x0005, 0x0006, &mut out);
    assert_eq!(at, 8);
    assert_eq!(out.get(7), Some(&1));
    assert_eq!(command.length(), 11);
}
