//! The tests of the Goodix fingerprint reader's logic.
//!
//! Every fixture here is written by hand: the HID report descriptor is a
//! byte array a test assembles, and so are the reports, because the point
//! of these tests is that the bytes are read the way the crate says they
//! are read, not that they came off a device.

use alloc::vec;
use alloc::vec::Vec;

use xaqu_usb_host::report;

use crate::descriptor::{Malformed as DescriptorMalformed, Report, SensorDescriptor};
use crate::packet::{Layout, LengthCovers, Malformed as PacketMalformed, Packet};
use crate::stream::{Frame, ImageLayout, Stream, StreamLayout};
use crate::vendor::{Builder, Error, VendorCommand, VendorTable};

/// A HID report descriptor as a fingerprint reader publishes it: a vendor
/// application collection, one input report of 64 bytes and one output
/// report of 8 bytes, both on report ids.
fn hid_descriptor() -> Vec<u8> {
    vec![
        0x06, 0x00, 0xFF, // Usage Page (vendor-defined)
        0x09, 0x01, // Usage (the application collection)
        0xA1, 0x01, // Collection (Application)
        0x85, 0x01, // Report ID (1)
        0x06, 0x00, 0xFF, // Usage Page (vendor-defined)
        0x09, 0x02, // Usage (the input report)
        0x15, 0x00, // Logical Minimum (0)
        0x25, 0xFF, // Logical Maximum (255)
        0x75, 0x08, // Report Size (8)
        0x95, 0x40, // Report Count (64)
        0x81, 0x02, // Input (Data, Variable, Absolute)
        0x85, 0x02, // Report ID (2)
        0x06, 0x00, 0xFF, // Usage Page (vendor-defined)
        0x09, 0x03, // Usage (the output report)
        0x75, 0x08, // Report Size (8)
        0x95, 0x08, // Report Count (8)
        0x91, 0x02, // Output (Data, Variable, Absolute)
        0xC0, // End Collection
    ]
}

/// The layout of a stream in these tests: three ids and a two-by-two
/// image.
fn stream_layout() -> StreamLayout {
    StreamLayout {
        start_id: 0x01,
        data_id: 0x02,
        end_id: 0x03,
        image: ImageLayout {
            width: 2,
            height: 2,
            dpi: 192,
        },
    }
}

#[test]
fn the_descriptor_says_what_the_sensor_sends() {
    let descriptor = SensorDescriptor::parse(&hid_descriptor()).expect("a good descriptor");
    assert_eq!(descriptor.application, Some((0xFF00, 0x0001)));
    assert!(descriptor.numbered);
    assert!(descriptor.vendor_application());
    assert_eq!(
        descriptor.inputs,
        vec![Report {
            id: 1,
            bytes: 64,
            bits: 512
        }]
    );
    assert_eq!(
        descriptor.outputs,
        vec![Report {
            id: 2,
            bytes: 8,
            bits: 64
        }]
    );
    assert_eq!(descriptor.input(1).map(|entry| entry.bytes), Some(64));
    assert_eq!(descriptor.output(2).map(|entry| entry.bytes), Some(8));
    assert!(descriptor.input(7).is_none());
}

#[test]
fn a_descriptor_without_an_output_report_is_refused() {
    let mut bytes = hid_descriptor();
    bytes.truncate(24); // The output report's items are gone.
    assert_eq!(
        SensorDescriptor::parse(&bytes),
        Err(DescriptorMalformed::NoOutputReport)
    );
}

#[test]
fn a_descriptor_without_an_input_report_is_refused() {
    let mut bytes = hid_descriptor();
    let _ = bytes.drain(7..24); // The input report's items are gone.
    assert_eq!(
        SensorDescriptor::parse(&bytes),
        Err(DescriptorMalformed::NoInputReport)
    );
}

#[test]
fn a_descriptor_that_is_not_a_hid_descriptor_is_refused() {
    assert_eq!(
        SensorDescriptor::parse(&[0x05]),
        Err(DescriptorMalformed::Hid(report::Error::Truncated))
    );
}

#[test]
fn the_number_of_reports_an_image_takes_is_derived() {
    let descriptor = SensorDescriptor::parse(&hid_descriptor()).expect("a good descriptor");
    assert_eq!(descriptor.packets_per_image(1024, 1), Some(16));
    assert_eq!(descriptor.packets_per_image(100, 1), Some(2));
    assert_eq!(descriptor.packets_per_image(10, 9), None);
    assert_eq!(descriptor.packets_per_image(0, 1), None);
}

#[test]
fn a_report_is_read_as_a_packet() {
    let bytes = [0x11, 0x03, 0xAA, 0xBB, 0xCC];
    let packet = Packet::parse(&bytes).expect("a good report");
    assert_eq!(packet.id, 0x11);
    assert_eq!(packet.payload, &[0xAA, 0xBB, 0xCC]);
    assert_eq!(packet.len(), 3);
    assert!(!packet.is_empty());
}

#[test]
fn a_report_shorter_than_its_header_is_refused() {
    assert_eq!(
        Packet::parse(&[0x11]),
        Err(PacketMalformed::TooShort { got: 1, need: 2 })
    );
    assert_eq!(
        Packet::parse(&[]),
        Err(PacketMalformed::TooShort { got: 0, need: 2 })
    );
}

#[test]
fn a_length_longer_than_the_report_is_refused() {
    assert_eq!(
        Packet::parse(&[0x11, 0x05, 0xAA]),
        Err(PacketMalformed::LengthMismatch {
            declared: 5,
            available: 1
        })
    );
}

#[test]
fn a_length_that_counts_the_whole_report_is_read() {
    let layout = Layout {
        length_size: 1,
        length_covers: LengthCovers::WholeReport,
    };
    let bytes = [0x11, 0x05, 0xAA, 0xBB, 0xCC];
    let packet = Packet::parse_with(&bytes, layout).expect("a good report");
    assert_eq!(packet.payload, &[0xAA, 0xBB, 0xCC]);
}

#[test]
fn a_wider_length_field_is_read() {
    let layout = Layout {
        length_size: 2,
        length_covers: LengthCovers::Payload,
    };
    let bytes = [0x11, 0x03, 0x00, 0xAA, 0xBB, 0xCC];
    let packet = Packet::parse_with(&bytes, layout).expect("a good report");
    assert_eq!(packet.id, 0x11);
    assert_eq!(packet.payload, &[0xAA, 0xBB, 0xCC]);
}

#[test]
fn a_length_field_too_wide_to_read_is_refused() {
    let layout = Layout {
        length_size: 5,
        length_covers: LengthCovers::Payload,
    };
    let bytes = [0x11, 0x01, 0x00, 0x00, 0x00, 0x00, 0xAA];
    assert_eq!(
        Packet::parse_with(&bytes, layout),
        Err(PacketMalformed::BadLayout)
    );
}

#[test]
fn an_unknown_packet_id_is_refused() {
    let bytes = [0x11, 0x01, 0xAA];
    assert_eq!(
        Packet::parse_known(&bytes, Layout::default(), &[0x22]),
        Err(PacketMalformed::UnknownPacket { id: 0x11 })
    );
    assert!(Packet::parse_known(&bytes, Layout::default(), &[0x11]).is_ok());
}

#[test]
fn an_image_is_reassembled_from_its_packets() {
    let mut stream = Stream::new(stream_layout());
    assert!(
        stream
            .push(&Packet {
                id: 0x01,
                payload: &[]
            })
            .is_none()
    );
    assert!(
        stream
            .push(&Packet {
                id: 0x02,
                payload: &[0x10, 0x11]
            })
            .is_none()
    );
    assert!(
        stream
            .push(&Packet {
                id: 0x02,
                payload: &[0x12, 0x13]
            })
            .is_none()
    );
    assert_eq!(stream.buffered(), 4);
    assert_eq!(stream.packets(), 2);
    let frame = stream
        .push(&Packet {
            id: 0x03,
            payload: &[],
        })
        .expect("the end of the image");
    assert_eq!(frame.data, vec![0x10, 0x11, 0x12, 0x13]);
    assert_eq!(frame.width, 2);
    assert_eq!(frame.height, 2);
    assert_eq!(frame.dpi, 192);
    assert!(frame.is_complete());
    assert_eq!(stream.buffered(), 0);
}

#[test]
fn the_start_packet_discards_what_came_before() {
    let mut stream = Stream::new(stream_layout());
    let _ = stream.push(&Packet {
        id: 0x01,
        payload: &[],
    });
    let _ = stream.push(&Packet {
        id: 0x02,
        payload: &[0xFF, 0xFF],
    });
    let _ = stream.push(&Packet {
        id: 0x01,
        payload: &[],
    });
    let _ = stream.push(&Packet {
        id: 0x02,
        payload: &[0x01],
    });
    let frame = stream
        .push(&Packet {
            id: 0x03,
            payload: &[],
        })
        .expect("the end of the image");
    assert_eq!(frame.data, vec![0x01]);
    assert!(!frame.is_complete());
}

#[test]
fn a_packet_of_another_kind_is_ignored() {
    let mut stream = Stream::new(stream_layout());
    assert!(
        stream
            .push(&Packet {
                id: 0x09,
                payload: &[0x01, 0x02]
            })
            .is_none()
    );
    assert_eq!(stream.buffered(), 0);
    assert_eq!(stream.packets(), 0);
}

#[test]
fn a_data_packet_before_the_start_is_ignored() {
    let mut stream = Stream::new(stream_layout());
    assert!(
        stream
            .push(&Packet {
                id: 0x02,
                payload: &[0x01]
            })
            .is_none()
    );
    assert_eq!(stream.buffered(), 0);
}

#[test]
fn a_frame_says_whether_it_is_whole() {
    let frame = Frame {
        width: 2,
        height: 2,
        dpi: 192,
        data: vec![0, 1, 2],
    };
    assert_eq!(frame.expected_bytes(), 4);
    assert!(!frame.is_complete());
}

#[test]
fn the_empty_table_refuses_every_command() {
    let table = VendorTable::empty();
    assert!(table.is_empty());
    assert_eq!(table.len(), 0);
    for command in VendorCommand::ALL {
        assert_eq!(
            table.request(command, &[]),
            Err(Error::UnknownOpcode(command))
        );
    }
}

#[test]
fn a_table_cannot_be_built_without_every_opcode() {
    assert_eq!(
        VendorTable::builder().build(),
        Err(Error::UnknownOpcode(VendorCommand::FingerPresence))
    );
    assert_eq!(
        VendorTable::builder()
            .opcode(VendorCommand::FingerPresence, 0x01)
            .build(),
        Err(Error::UnknownOpcode(VendorCommand::CaptureStart))
    );
}

#[test]
fn a_table_with_every_opcode_makes_a_request() {
    let mut builder = Builder::default();
    for (index, command) in VendorCommand::ALL.iter().enumerate() {
        builder = builder.opcode(*command, 0x40 + index as u8);
    }
    let table = builder.build().expect("every opcode is known");
    assert!(!table.is_empty());
    assert_eq!(table.len(), VendorCommand::ALL.len());
    let request = table
        .request(VendorCommand::Match, &[0xAA])
        .expect("the match opcode is known");
    assert_eq!(request.command, VendorCommand::Match);
    assert_eq!(request.opcode, 0x44);
    assert_eq!(request.payload, vec![0xAA]);
}
