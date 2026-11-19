//! The host tests: a register block of bytes, a memory of bytes, and a
//! codec of answers.
//!
//! The fake register block and the fake memory are arrays the tests write
//! into, so the driver's own reads and writes are checked against the
//! specification's offsets and bits.

use crate::bdl;
use crate::codec;
use crate::control;
use crate::controller::{self, register, stream, Mmio};
use crate::corb::{self, Memory};
use crate::path;

/// The register block's bytes.
const BLOCK: usize = 4096;

/// A register block of bytes.
struct Fake {
    bytes: [u8; BLOCK],
}

impl Fake {
    fn new() -> Self {
        Self {
            bytes: [0; BLOCK],
        }
    }
}

impl Mmio for Fake {
    fn read8(&self, offset: u32) -> u8 {
        self.bytes.get(offset as usize).copied().unwrap_or_default()
    }

    fn read16(&self, offset: u32) -> u16 {
        let low = u16::from(self.read8(offset));
        let high = u16::from(self.read8(offset + 1));
        low | (high << 8)
    }

    fn read32(&self, offset: u32) -> u32 {
        let low = u32::from(self.read16(offset));
        let high = u32::from(self.read16(offset + 2));
        low | (high << 16)
    }

    fn write8(&mut self, offset: u32, value: u8) {
        if let Some(slot) = self.bytes.get_mut(offset as usize) {
            *slot = value;
        }
    }

    fn write16(&mut self, offset: u32, value: u16) {
        self.write8(offset, (value & 0xFF) as u8);
        self.write8(offset + 1, (value >> 8) as u8);
    }

    fn write32(&mut self, offset: u32, value: u32) {
        self.write16(offset, (value & 0xFFFF) as u16);
        self.write16(offset + 2, (value >> 16) as u16);
    }
}

/// A memory of bytes, as the controller's DMA sees it.
struct FakeMemory {
    bytes: [u8; 4096],
}

impl FakeMemory {
    fn new() -> Self {
        Self {
            bytes: [0; 4096],
        }
    }
}

impl Memory for FakeMemory {
    fn read32(&self, offset: u32) -> u32 {
        let at = offset as usize;
        let low = u32::from(self.bytes.get(at).copied().unwrap_or_default());
        let mid = u32::from(self.bytes.get(at + 1).copied().unwrap_or_default());
        let high = u32::from(self.bytes.get(at + 2).copied().unwrap_or_default());
        let top = u32::from(self.bytes.get(at + 3).copied().unwrap_or_default());
        low | (mid << 8) | (high << 16) | (top << 24)
    }

    fn write32(&mut self, offset: u32, value: u32) {
        let at = offset as usize;
        let bytes = value.to_le_bytes();
        for (index, byte) in bytes.iter().enumerate() {
            if let Some(slot) = self.bytes.get_mut(at + index) {
                *slot = *byte;
            }
        }
    }
}

#[test]
fn a_twelve_bit_verb_carries_its_parts() {
    let verb = corb::verb12(0, 0x1A, 0xF0, 0x09);
    assert_eq!(verb >> 28, 0, "the codec address");
    assert_eq!((verb >> 20) & 0xFF, 0x1A, "the node address");
    assert_eq!((verb >> 8) & 0xFF, 0xF0, "the parameter identifier");
    assert_eq!(verb & 0xFF, 0x09, "the parameter's value");
}

#[test]
fn a_four_bit_verb_carries_a_sixteen_bit_value() {
    let verb = corb::verb4(1, 0x02, 0x03, 0x1234);
    assert_eq!(verb >> 28, 1, "the codec address");
    assert_eq!((verb >> 20) & 0xFF, 0x02, "the node address");
    assert_eq!((verb >> 16) & 0x0F, 0x03, "the parameter identifier");
    assert_eq!(verb & 0xFFFF, 0x1234, "the parameter's value");
}

#[test]
fn the_verbs_are_the_specification_s_own() {
    assert_eq!(
        codec::get_parameter(0, 0x1A, codec::parameter::WIDGET_CAP),
        0x01A0_F009,
        "the widget capabilities verb"
    );
    assert_eq!(
        codec::set_power(0, 0x18, 0),
        0x0180_7005,
        "the power verb"
    );
    assert_eq!(
        codec::set_pin_control(0, 0x19, codec::PINCTL_OUT_EN),
        0x0190_7047,
        "the pin control verb"
    );
    assert_eq!(
        codec::set_amp_gain(0, 0x02, 0x10, false),
        0x0023_3010,
        "the amplifier verb"
    );
}

#[test]
fn the_controller_comes_out_of_its_reset() {
    let mut fake = Fake::new();
    fake.write16(register::STATESTS, 1);
    let mut controller = controller::Controller::new(fake);
    assert!(controller.codec_ready(0), "the codec is ready");
    let outcome = controller.reset(4);
    assert_eq!(outcome, Ok(()), "the reset finished");
    assert_eq!(
        controller.mmio().read32(register::GCTL) & controller::GCTL_RESET,
        0,
        "the reset bit is clear"
    );
}

#[test]
fn a_codec_that_stays_in_its_reset_is_an_error() {
    let fake = Fake::new();
    let mut controller = controller::Controller::new(fake);
    let outcome = controller.reset(3);
    assert_eq!(outcome, Err(controller::Error::Reset), "the reset did not finish");
}

#[test]
fn the_controller_says_how_many_streams_it_has() {
    let mut fake = Fake::new();
    fake.write16(register::GCAP, 0x4401);
    let controller = controller::Controller::new(fake);
    assert_eq!(controller.input_streams(), 4, "the input streams");
    assert_eq!(controller.output_streams(), 4, "the output streams");
}

#[test]
fn the_rings_are_told_where_they_live() {
    let mut fake = Fake::new();
    fake.write32(register::CORBLBASE, 0x1000);
    fake.write32(register::CORBUBASE, 0);
    fake.write32(register::RIRBLBASE, 0x2000);
    fake.write32(register::RIRBUBASE, 0);
    let controller = controller::Controller::new(fake);
    assert_eq!(controller.corb_base(), 0x1000, "the CORB's base");
    assert_eq!(controller.rirb_base(), 0x2000, "the RIRB's base");
}

#[test]
fn the_corb_takes_a_verb_and_moves_its_pointer() {
    let mut memory = FakeMemory::new();
    let verb = corb::verb12(0, 0x1A, 0xF0, 0x09);
    {
        let mut ring = corb::Corb::new(&mut memory, 4);
        assert!(ring.has_room(), "the CORB has room");
        let pushed = ring.push(verb);
        assert_eq!(pushed, Ok(1), "the verb went into the second entry");
        assert_eq!(ring.write_pointer(), 2, "the pointer moved");
    }
    assert_eq!(memory.read32(4), verb, "the verb is in the ring");
}

#[test]
fn a_full_corb_refuses_a_verb() {
    let mut memory = FakeMemory::new();
    let mut ring = corb::Corb::new(&mut memory, 4);
    for _ in 0..3 {
        let pushed = ring.push(corb::verb12(0, 0, 0, 0));
        assert_eq!(pushed.is_ok(), true, "the verb went in");
    }
    assert!(!ring.has_room(), "the CORB is full");
    let pushed = ring.push(corb::verb12(0, 0, 0, 0));
    assert_eq!(pushed, Err(corb::CorbError::Full), "the CORB is full");
}

#[test]
fn the_rirb_answers_are_read_from_the_ring() {
    let mut memory = FakeMemory::new();
    memory.write32(0, 0xA5A5_0000);
    memory.write32(4, 0x0000_0001);
    let ring = corb::Rirb::new(&memory, 4);
    assert_eq!(ring.answer(0), Some(0xA5A5_0000), "the answer");
    assert_eq!(ring.extra(0), Some(1), "the marks");
    assert_eq!(ring.codec(0), Some(1), "the codec");
    assert!(!ring.unsolicited(0), "the answer was asked for");
    assert_eq!(ring.answer(4), None, "past the ring");
}

#[test]
fn a_widget_says_what_it_is() {
    assert_eq!(codec::NodeType::of(0 << 20), codec::NodeType::Output, "an out");
    assert_eq!(codec::NodeType::of(1 << 20), codec::NodeType::Input, "an in");
    assert_eq!(codec::NodeType::of(2 << 20), codec::NodeType::Mixer, "a mixer");
    assert_eq!(codec::NodeType::of(4 << 20), codec::NodeType::Pin, "a pin");
    assert_eq!(codec::NodeType::of(0x0F << 20), codec::NodeType::Other, "other");
}

#[test]
fn a_pin_says_what_it_can_do() {
    let capabilities = codec::PinCapabilities::of(0b0001_1100);
    assert!(!capabilities.impedance_sense, "the impedance bit");
    assert!(!capabilities.trigger_required, "the trigger bit");
    assert!(capabilities.presence_detect, "the presence bit");
    assert!(capabilities.headphone_drive, "the headphone bit");
    assert!(capabilities.output_capable, "the output bit");
    assert!(!capabilities.input_capable, "the input bit");
}

#[test]
fn a_configuration_default_says_which_device_it_is_for() {
    let speaker = codec::ConfigDefault::of(1 << 20);
    assert_eq!(speaker.device, codec::Jack::Speaker, "a speaker");
    let headphone = codec::ConfigDefault::of(2 << 20);
    assert_eq!(headphone.device, codec::Jack::Headphone, "headphones");
    let microphone = codec::ConfigDefault::of(0x0A << 20);
    assert_eq!(microphone.device, codec::Jack::Microphone, "a microphone");
    let association = codec::ConfigDefault::of(3 << 4);
    assert_eq!(association.association, 3, "the association");
}

#[test]
fn a_connection_list_is_two_to_a_byte() {
    let connections = codec::Connections::unpack(&[0x21, 0x00], 4);
    assert_eq!(connections.ids(), &[0x01, 0x02, 0x00, 0x00], "the nodes");
    assert!(connections.contains(0x02), "the second node");
    assert!(!connections.contains(0x03), "a node that is not there");
}

#[test]
fn a_connection_list_ends_where_it_is_told() {
    let connections = codec::Connections::unpack(&[0x01, 0x80], 1);
    assert_eq!(connections.ids(), &[0x01], "only the first node");
}

#[test]
fn an_amplifier_says_what_it_can_do() {
    let capabilities = codec::AmpCapabilities::of(0x8000_0280);
    assert_eq!(capabilities.offset, 0x00, "the offset");
    assert_eq!(capabilities.steps, 0x02, "the steps");
    assert_eq!(capabilities.step_size, 0x00, "the step size");
    assert!(capabilities.mute, "the mute bit");
}

/// A codec of three nodes: a converter, a mixer and a pin.
struct FakeCodec {
    connections: [codec::Connections; 3],
}

impl FakeCodec {
    fn new() -> Self {
        let mut connections = [codec::Connections::default(); 3];
        connections[0] = codec::Connections::unpack(&[0x01], 1);
        connections[1] = codec::Connections::unpack(&[0x02], 1);
        Self { connections }
    }
}

impl path::Nodes for FakeCodec {
    fn count(&self) -> u8 {
        3
    }

    fn kind(&self, id: u8) -> codec::NodeType {
        match id {
            0 => codec::NodeType::Output,
            1 => codec::NodeType::Mixer,
            _ => codec::NodeType::Pin,
        }
    }

    fn connections(&self, id: u8) -> codec::Connections {
        self.connections
            .get(id as usize)
            .copied()
            .unwrap_or_default()
    }
}

#[test]
fn the_path_to_a_pin_is_found() {
    let codec = FakeCodec::new();
    let found = path::path(&codec, 2);
    let expected = path::Path {
        converter: 0,
        amplifier: None,
        mixer: Some(1),
        pin: 2,
    };
    assert_eq!(found, Some(expected), "the path");
}

#[test]
fn a_pin_serves_the_device_it_is_for() {
    let capabilities = codec::PinCapabilities::of(1 << 4);
    let speaker = codec::ConfigDefault::of(1 << 20);
    assert!(
        path::serves(&capabilities, &speaker, codec::Jack::Speaker),
        "the speaker"
    );
    assert!(
        !path::serves(&capabilities, &speaker, codec::Jack::Headphone),
        "not headphones"
    );
}

#[test]
fn a_descriptor_carries_its_address_and_length() {
    let entry = bdl::Entry {
        address: 0x1234_5678_9ABC_DEF0,
        bytes: 0x0100,
        interrupt: true,
    };
    let bytes = entry.encode();
    assert_eq!(bytes.len(), bdl::ENTRY_BYTES, "the length");
    assert_eq!(bdl::Entry::decode(&bytes), Some(entry), "the round trip");
}

#[test]
fn a_descriptor_without_the_flag_says_so() {
    let entry = bdl::Entry {
        address: 0,
        bytes: 0x2000,
        interrupt: false,
    };
    let bytes = entry.encode();
    assert_eq!(bytes.get(8).copied(), Some(0x00), "the length's byte");
    assert_eq!(bytes.get(12).copied(), Some(0x00), "the flags' byte");
}

#[test]
fn the_start_message_carries_the_device() {
    let start = control::Start {
        version: 1,
        location: 0x0000_0100,
        pci_device_id: 0xA0C8,
        bar: 0,
    };
    let bytes = start.encode();
    assert_eq!(control::Start::decode(&bytes), Some(start), "the round trip");
    assert_eq!(
        control::Start::decode(&bytes.get(..8).unwrap_or_default()),
        None,
        "short"
    );
}

#[test]
fn the_streams_are_where_the_specification_says() {
    assert_eq!(
        controller::Controller::<Fake>::stream_base(0),
        0x80,
        "the first input"
    );
    assert_eq!(
        controller::Controller::<Fake>::stream_base(4),
        0x100,
        "the first output"
    );
    assert_eq!(
        controller::Controller::<Fake>::stream_base(7),
        0x160,
        "the last output"
    );
    assert_eq!(stream::FORMAT, 0x12, "the format register");
    assert_eq!(stream::BDLPL, 0x18, "the list's base");
    assert_eq!(controller::FORMAT_48K_S16_2CH, 0x0012, "the format");
}
