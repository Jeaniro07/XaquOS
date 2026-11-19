//! The host tests: the bytes a firmware file carries, and the bytes a
//! driver has to make of a command and its answer.
//!
//! Every fixture here is a buffer the test writes itself, so a test that
//! passes says that the crate reads the kernel's formats the way the kernel
//! writes them.

use crate::cmd::{
    self, CmdHeader, CommandGroup, CommandId, WideCmdHeader, ALIVE_STATUS_ERR, ALIVE_STATUS_OK,
};
use crate::device::{
    self, firmware_file_name, lookup, lookup_by_pci, DeviceFamily, INTEL_VENDOR_ID,
};
use crate::firmware::{
    self, Error as FirmwareError, FirmwareFile, TlvType, UcodeFlags, HEADER_LEN, TLV_HEADER_LEN,
};
use std::vec;
use std::vec::Vec;

/// The version word of the fixture: major 45, minor 0, API 70, serial 1.
const FIXTURE_VERSION: u32 = 0x2d_00_46_01;

/// A firmware file with the given records in it.
fn firmware_file(records: &[(u32, &[u8])]) -> Vec<u8> {
    let mut out = vec![0_u8; HEADER_LEN];
    out[4..8].copy_from_slice(&firmware::UCODE_MAGIC.to_le_bytes());
    let readable = b"Intel(R) Firmware 46.123";
    out[8..8 + readable.len()].copy_from_slice(readable);
    out[72..76].copy_from_slice(&FIXTURE_VERSION.to_le_bytes());
    out[76..80].copy_from_slice(&0x0000_abcd_u32.to_le_bytes());
    for (kind, data) in records {
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }
    out
}

#[test]
fn a_firmware_header_reads_its_parts() {
    let file = firmware_file(&[]);
    let header = FirmwareFile::parse(&file)
        .expect("the fixture is a firmware file")
        .header()
        .clone();
    assert_eq!(header.major(), 45, "the major version");
    assert_eq!(header.minor(), 0, "the minor version");
    assert_eq!(header.api(), 70, "the API version");
    assert_eq!(header.serial(), 1, "the serial number");
    assert_eq!(header.build(), 0x0000_abcd, "the build number");
    assert_eq!(
        header.human_readable(),
        "Intel(R) Firmware 46.123",
        "the human readable version"
    );
}

#[test]
fn a_firmware_directory_reads_its_records() {
    let file = firmware_file(&[
        (TlvType::Inst.raw(), &[0xAA, 0xBB, 0xCC]),
        (TlvType::Flags.raw(), &[0x01, 0x00, 0x00, 0x00]),
        (TlvType::FirmwareVersion.raw(), b"46.123"),
    ]);
    let parsed = FirmwareFile::parse(&file).expect("the fixture parses");
    let kinds: Vec<u32> = parsed
        .tlvs()
        .map(|entry| entry.expect("every record is well formed").kind().raw())
        .collect();
    assert_eq!(
        kinds,
        vec![
            TlvType::Inst.raw(),
            TlvType::Flags.raw(),
            TlvType::FirmwareVersion.raw()
        ],
        "the records in the order they were written"
    );
    let inst = parsed.tlv(TlvType::Inst).expect("the instructions are there");
    assert_eq!(inst, &[0xAA, 0xBB, 0xCC], "the record's data");
}

#[test]
fn a_record_of_a_known_type_is_found() {
    let file = firmware_file(&[
        (TlvType::ProbeMaxLen.raw(), &200_u32.to_le_bytes()),
        (TlvType::NumScanChannels.raw(), &11_u32.to_le_bytes()),
    ]);
    let parsed = FirmwareFile::parse(&file).expect("the fixture parses");
    assert_eq!(
        parsed.tlv_u32(TlvType::NumScanChannels),
        Some(11),
        "the scan channel count"
    );
    assert_eq!(
        parsed.tlv_u32(TlvType::ProbeMaxLen),
        Some(200),
        "the maximum probe length"
    );
    assert_eq!(
        parsed.tlv(TlvType::Inst),
        None,
        "a record that is not there"
    );
}

#[test]
fn a_truncated_header_is_an_error() {
    let file = firmware_file(&[]);
    let short = &file[..HEADER_LEN - 1];
    match FirmwareFile::parse(short) {
        Err(FirmwareError::TruncatedHeader { needed, found }) => {
            assert_eq!(needed, HEADER_LEN, "the length the header needs");
            assert_eq!(found, HEADER_LEN - 1, "the length the buffer has");
        }
        other => panic!("expected a truncated header, got {other:?}"),
    }
}

#[test]
fn a_bad_magic_is_an_error() {
    let mut file = firmware_file(&[]);
    file[4..8].copy_from_slice(&0xdead_beef_u32.to_le_bytes());
    match FirmwareFile::parse(&file) {
        Err(FirmwareError::BadMagic { found }) => {
            assert_eq!(found, 0xdead_beef, "the word that was found");
        }
        other => panic!("expected a bad magic, got {other:?}"),
    }
}

#[test]
fn the_old_layout_is_not_mistaken_for_the_tlv_one() {
    let mut file = firmware_file(&[]);
    file[0..4].copy_from_slice(&0x0000_0001_u32.to_le_bytes());
    assert_eq!(
        FirmwareFile::parse(&file),
        Err(FirmwareError::LegacyLayout),
        "a first word that is not zero is the older layout"
    );
}

#[test]
fn a_record_that_claims_too_much_is_an_error() {
    let mut file = firmware_file(&[(TlvType::Inst.raw(), &[0x01, 0x02])]);
    let last = file.len();
    file[last - 2..last].copy_from_slice(&999_u32.to_le_bytes());
    let parsed = FirmwareFile::parse(&file).expect("the header still parses");
    let mut entries = parsed.tlvs();
    match entries.next() {
        Some(Err(FirmwareError::BadTlvLength { offset, length })) => {
            assert_eq!(offset, HEADER_LEN, "where the record starts");
            assert_eq!(length, 999, "the length the record claims");
        }
        other => panic!("expected a bad record length, got {other:?}"),
    }
}

#[test]
fn the_flag_bits_are_the_kernel_s() {
    let file = firmware_file(&[(
        TlvType::Flags.raw(),
        &(UcodeFlags::MFP.raw() | UcodeFlags::UAPSD_SUPPORT.raw()).to_le_bytes(),
    )]);
    let parsed = FirmwareFile::parse(&file).expect("the fixture parses");
    let raw = parsed
        .tlv_u32(TlvType::Flags)
        .expect("the flags record is there");
    let flags = UcodeFlags(raw);
    assert!(flags.contains(UcodeFlags::MFP), "management frame protection");
    assert!(
        flags.contains(UcodeFlags::UAPSD_SUPPORT),
        "unannounced power save"
    );
    assert!(
        !flags.contains(UcodeFlags::PAN),
        "PAN is not in the bitmap"
    );
}

#[test]
fn a_short_header_round_trips() {
    let header = CmdHeader::new(cmd::legacy::NVM_ACCESS_CMD, 0x0, 0x1234);
    let bytes = header.encode();
    assert_eq!(bytes.len(), 4, "the short header is four bytes");
    assert_eq!(bytes[0], cmd::legacy::NVM_ACCESS_CMD, "the opcode");
    assert_eq!(bytes[1], 0x0, "the group");
    assert_eq!(bytes[2], 0x34, "the sequence, little endian");
    assert_eq!(bytes[3], 0x12, "the sequence, little endian");
    let parsed = CmdHeader::parse(&bytes).expect("the header parses");
    assert_eq!(parsed, header, "the header round trips");
}

#[test]
fn a_wide_header_round_trips() {
    let header = WideCmdHeader::new(0x21, 0x02, 0xbeef, 0x40, 0, 3);
    let bytes = header.encode();
    assert_eq!(bytes.len(), 8, "the wide header is eight bytes");
    let parsed = WideCmdHeader::parse(&bytes).expect("the header parses");
    assert_eq!(parsed, header, "the header round trips");
    assert_eq!(parsed.length(), 0x40, "the length of the payload");
    assert_eq!(parsed.version(), 3, "the version");
}

#[test]
fn a_truncated_header_is_an_error() {
    let bytes = [0x01_u8, 0x02, 0x03];
    assert_eq!(
        CmdHeader::parse(&bytes),
        Err(cmd::Error::Truncated {
            needed: 4,
            found: 3
        }),
        "three bytes are not a header"
    );
}

#[test]
fn a_command_is_framed_into_bytes() {
    let id = CommandId::new(cmd::legacy::PHY_CONFIGURATION_CMD, CommandGroup::Legacy, 0);
    let payload = [0x01_u8, 0x02, 0x03, 0x04];
    let mut out = [0_u8; 16];
    let written = cmd::frame_command(id, 0x0200, &payload, &mut out).expect("it fits");
    assert_eq!(written, 8 + 4, "the header and the payload");
    let header = WideCmdHeader::parse(&out).expect("the header parses");
    assert_eq!(header.cmd(), cmd::legacy::PHY_CONFIGURATION_CMD, "the opcode");
    assert_eq!(header.length(), 4, "the length of the payload");
    assert_eq!(&out[8..12], &payload, "the payload");
}

#[test]
fn a_command_that_does_not_fit_is_an_error() {
    let id = CommandId::new(1, CommandGroup::System, 0);
    let payload = [0_u8; 8];
    let mut out = [0_u8; 8];
    match cmd::frame_command(id, 0, &payload, &mut out) {
        Err(cmd::Error::PayloadTooLong { len, capacity }) => {
            assert_eq!(len, 16, "the bytes the command needs");
            assert_eq!(capacity, 8, "the bytes the buffer has");
        }
        other => panic!("expected a payload that does not fit, got {other:?}"),
    }
}

#[test]
fn a_response_carries_its_status() {
    let mut response = [0_u8; 12];
    response[8..12].copy_from_slice(&0x0000_0000_u32.to_le_bytes());
    let (header, payload) = cmd::parse_response(&response).expect("the response parses");
    assert_eq!(payload.len(), 4, "the payload");
    assert_eq!(header.length(), 4, "the length the header declares");
    assert_eq!(
        cmd::response_status(&response).expect("the status reads"),
        0,
        "the status word"
    );
}

#[test]
fn a_response_whose_length_disagrees_is_an_error() {
    let mut response = [0_u8; 12];
    response[8..12].copy_from_slice(&0x0000_0000_u32.to_le_bytes());
    match cmd::parse_response(&response) {
        Err(cmd::Error::LengthMismatch { declared, found }) => {
            assert_eq!(declared, 0, "the length the header declares");
            assert_eq!(found, 4, "the bytes that are there");
        }
        other => panic!("expected a length mismatch, got {other:?}"),
    }
}

#[test]
fn a_command_id_round_trips() {
    let id = CommandId::new(0x21, CommandGroup::System, 2);
    let raw = id.raw();
    assert_eq!(raw & 0xff, 0x21, "the opcode sits in the low byte");
    assert_eq!((raw >> 8) & 0xff, 0x02, "the group sits above it");
    assert_eq!((raw >> 16) & 0xff, 0x02, "the version sits above that");
    assert_eq!(CommandId::from_raw(raw), id, "the id round trips");
    assert_eq!(
        id.group_kind(),
        Some(CommandGroup::System),
        "the group is one the firmware names"
    );
}

#[test]
fn the_sequence_number_holds_the_queue_and_the_index() {
    let sequence = cmd::queue_to_seq(3) | 0x25;
    assert_eq!(cmd::seq_to_queue(sequence), 3, "the queue");
    assert_eq!(cmd::seq_to_index(sequence), 0x25, "the index");
    assert_eq!(cmd::SEQ_RX_FRAME, 0x8000, "the firmware's own bit");
}

#[test]
fn the_alive_status_is_the_kernel_s() {
    assert_eq!(ALIVE_STATUS_OK, 0xcafe, "the good status");
    assert_eq!(ALIVE_STATUS_ERR, 0xdead, "the bad status");
}

#[test]
fn the_ac_9560_is_in_the_table() {
    let device = lookup(0x9df0).expect("the AC 9560 is there");
    assert_eq!(device.name, "Intel(R) Wireless-AC 9560", "the name");
    assert_eq!(device.family, DeviceFamily::Iwl9000, "the family");
    assert_eq!(
        device.firmware_prefix,
        "iwlwifi-9000-pu-b0-jf-b0",
        "the firmware prefix"
    );
    assert_eq!(device.ucode_api_min, 30, "the lowest API version");
    assert_eq!(device.ucode_api_max, 46, "the highest API version");
}

#[test]
fn the_cnvi_part_is_in_the_table() {
    let device = lookup(0xa0f0).expect("the CNVi part is there");
    assert_eq!(device.family, DeviceFamily::Qu, "the family");
    assert_eq!(
        device.firmware_prefix,
        "iwlwifi-Qu-b0-hr-b0",
        "the firmware prefix"
    );
}

#[test]
fn a_device_that_is_not_in_the_table_is_refused() {
    assert!(lookup(0xffff).is_none(), "an unknown device id");
    assert!(
        lookup_by_pci(0x1234, 0x9df0).is_none(),
        "a vendor that is not Intel"
    );
    assert!(
        lookup_by_pci(INTEL_VENDOR_ID, 0x9df0).is_some(),
        "Intel's own device id"
    );
}

#[test]
fn the_firmware_file_name_is_built_the_way_the_driver_builds_it() {
    let name = firmware_file_name("iwlwifi-9000-pu-b0-jf-b0", 46);
    assert_eq!(
        name.as_str(),
        "iwlwifi-9000-pu-b0-jf-b0-46.ucode",
        "the file name"
    );
    let small = firmware_file_name("iwlwifi-Qu-b0-hr-b0", 7);
    assert_eq!(small.as_str(), "iwlwifi-Qu-b0-hr-b0-7.ucode", "one digit");
}

#[test]
fn the_published_firmware_versions_are_the_ones_shipped() {
    assert!(
        device::PUBLISHED_9000_PU_APIS.contains(&46),
        "the 9000 family ships API 46"
    );
    assert!(
        device::PUBLISHED_QU_APIS.contains(&59),
        "the Qu family ships API 59"
    );
    assert!(
        !device::PUBLISHED_9000_PU_APIS.contains(&99),
        "and nothing else"
    );
}

#[test]
fn the_tlv_header_is_eight_bytes() {
    assert_eq!(TLV_HEADER_LEN, 8, "the length of a record header");
}
