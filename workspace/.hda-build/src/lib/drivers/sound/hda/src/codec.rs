//! What a codec says about its nodes.
//!
//! A codec is a tree of nodes: an amplifier, a mixer, a pin that a jack is
//! wired to, a digital converter. The driver asks about them with the
//! codec's verbs -- each is a parameter read, whose answer comes back in the
//! RIRB -- and this module is what the answers mean.
//!
//! The parameter identifiers, the verbs and the meaning of the bits are the
//! Intel High Definition Audio specification's chapter 3, as QEMU's
//! `hw/audio/intel-hda-defs.h` and Linux's `sound/hda_codec.h` read them.

use crate::corb::{verb12, verb4};

/// The codec's parameter identifiers, as the specification numbers them.
pub mod parameter {
    /// The vendor identifier.
    pub const VENDOR_ID: u8 = 0x00;
    /// The subsystem identifier.
    pub const SUBSYSTEM_ID: u8 = 0x01;
    /// The revision identifier.
    pub const REVISION_ID: u8 = 0x02;
    /// How many addressable nodes the codec has.
    pub const NODE_COUNT: u8 = 0x04;
    /// The function group's type.
    pub const FUNCTION_TYPE: u8 = 0x05;
    /// The audio function group's capabilities.
    pub const AUDIO_FG_CAP: u8 = 0x08;
    /// What the node is and what it can do.
    pub const WIDGET_CAP: u8 = 0x09;
    /// The PCM formats the node takes.
    pub const PCM: u8 = 0x0A;
    /// The streams the node takes.
    pub const STREAM: u8 = 0x0B;
    /// What the pin can do and what it can sense.
    pub const PIN_CAP: u8 = 0x0C;
    /// The input amplifier's capabilities.
    pub const AMP_IN_CAP: u8 = 0x0D;
    /// How long the node's connection list is.
    pub const CONNLIST_LEN: u8 = 0x0E;
    /// The node's connection list.
    pub const CONNECTION_LIST: u8 = 0x0F;
    /// The node's power state.
    pub const POWER_STATE: u8 = 0x0F;
    /// The processing capabilities.
    pub const PROC_CAP: u8 = 0x10;
    /// The general purpose input and output capabilities.
    pub const GPIO_CAP: u8 = 0x11;
    /// The output amplifier's capabilities.
    pub const AMP_OUT_CAP: u8 = 0x12;
    /// The volume knob's capabilities.
    pub const VOL_KNB_CAP: u8 = 0x13;
}

/// The verb that reads a parameter: the identifier goes in its payload.
pub const GET_PARAMETER: u8 = 0xF0;
/// The verb that reads a node's configuration default.
pub const GET_CONFIG_DEFAULT: u8 = 0xF0;
/// The configuration default's own identifier in the verb's payload.
pub const CONFIG_DEFAULT: u8 = 0x1C;

/// The verb that sets a node's power state.
pub const SET_POWER: u8 = 0x70;
/// The verb that sets a pin's amplifier state.
pub const SET_EAPD: u8 = 0x70;
/// The verb that sets a pin's widget control.
pub const SET_PIN_CONTROL: u8 = 0x70;
/// The verb that chooses a pin's connection.
pub const SET_CONNECT_SELECT: u8 = 0x70;

/// The pin control register's input enable bit.
pub const PINCTL_IN_EN: u8 = 1 << 5;
/// The pin control register's output enable bit.
pub const PINCTL_OUT_EN: u8 = 1 << 6;
/// The pin control register's headphone amplifier bit.
pub const PINCTL_HP_EN: u8 = 1 << 7;

/// The verb that reads a parameter of the codec `cad`, node `nid`.
#[must_use]
pub fn get_parameter(cad: u8, nid: u8, parameter: u8) -> u32 {
    verb12(cad, nid, GET_PARAMETER, parameter)
}

/// The verb that reads the configuration default of the codec `cad`, node
/// `nid`.
#[must_use]
pub fn get_config_default(cad: u8, nid: u8) -> u32 {
    verb12(cad, nid, GET_CONFIG_DEFAULT, CONFIG_DEFAULT)
}

/// The verb that sets the power state of the codec `cad`, node `nid`.
#[must_use]
pub fn set_power(cad: u8, nid: u8, state: u8) -> u32 {
    verb12(cad, nid, SET_POWER, 0x05 | (state & 0x07))
}

/// The verb that sets the amplifier state of the codec `cad`, node `nid`.
///
/// The amplifier is enabled by the value's second bit, as the
/// specification's "Set EAPD/BTL Enable" verb has it.
#[must_use]
pub fn set_eapd(cad: u8, nid: u8, on: bool) -> u32 {
    let value = 0x0C | if on { 0x02 } else { 0x00 };
    verb12(cad, nid, SET_EAPD, value)
}

/// The verb that sets the widget control of the codec `cad`, node `nid`.
#[must_use]
pub fn set_pin_control(cad: u8, nid: u8, control: u8) -> u32 {
    verb12(cad, nid, SET_PIN_CONTROL, 0x07 | control)
}

/// The verb that chooses the connection of the codec `cad`, node `nid`.
#[must_use]
pub fn set_connection_select(cad: u8, nid: u8, index: u8) -> u32 {
    verb12(cad, nid, SET_CONNECT_SELECT, 0x01 | index)
}

/// The verb that sets the gain of the amplifier of the codec `cad`, node
/// `nid`.
///
/// The amplifier's gain is the value's low seven bits and its mute the
/// bit above them; the bit above that says which side of the amplifier the
/// gain is for.
#[must_use]
pub fn set_amp_gain(cad: u8, nid: u8, gain: u8, mute: bool) -> u32 {
    let mut value = u16::from(gain & 0x7F);
    if mute {
        value |= 0x0080;
    }
    value |= 0x3000;
    verb4(cad, nid, 0x03, value)
}

/// The verb that sets the format of the codec `cad`, node `nid`.
#[must_use]
pub fn set_stream_format(cad: u8, nid: u8, format: u16) -> u32 {
    verb4(cad, nid, 0x02, format)
}

/// What a node is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeType {
    /// An output: something the codec plays out of.
    Output,
    /// An input: something that plays into the codec.
    Input,
    /// A mixer.
    Mixer,
    /// A selector.
    Selector,
    /// A pin: what a jack is wired to.
    Pin,
    /// A power node.
    Power,
    /// A volume knob.
    VolumeKnob,
    /// A beep generator.
    Beep,
    /// Something the driver does not know.
    Other,
}

impl NodeType {
    /// The node's type, as the widget capabilities word carries it.
    ///
    /// The type is the word's bits 23:20.
    #[must_use]
    pub fn of(capabilities: u32) -> Self {
        match (capabilities >> 20) & 0x0F {
            0x00 => Self::Output,
            0x01 => Self::Input,
            0x02 => Self::Mixer,
            0x03 => Self::Selector,
            0x04 => Self::Pin,
            0x05 => Self::Power,
            0x06 => Self::VolumeKnob,
            0x07 => Self::Beep,
            _ => Self::Other,
        }
    }
}

/// What a node's amplifier can do.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct AmpCapabilities {
    /// The offset of the amplifier's zero, in 1/256 dB.
    pub offset: u8,
    /// How many steps the amplifier has.
    pub steps: u8,
    /// The size of a step, in 1/4 dB.
    pub step_size: u8,
    /// Whether the amplifier can be muted.
    pub mute: bool,
}

impl AmpCapabilities {
    /// The amplifier's capabilities, as the amplifier capability parameter's
    /// answer.
    #[must_use]
    pub fn of(answer: u32) -> Self {
        Self {
            offset: (answer & 0x7F) as u8,
            steps: ((answer >> 8) & 0x7F) as u8,
            step_size: ((answer >> 16) & 0x7F) as u8,
            mute: answer & (1 << 31) != 0,
        }
    }
}

/// What a pin can do and what it can sense.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PinCapabilities {
    /// The pin senses its jack's impedance.
    pub impedance_sense: bool,
    /// The pin needs a trigger before it reports a change.
    pub trigger_required: bool,
    /// The pin senses whether a jack is plugged in.
    pub presence_detect: bool,
    /// The pin can drive a headphone.
    pub headphone_drive: bool,
    /// The pin plays sound out.
    pub output_capable: bool,
    /// The pin takes sound in.
    pub input_capable: bool,
    /// The pin is balanced.
    pub balanced: bool,
    /// The pin drives an amplifier.
    pub amplifier_capable: bool,
}

impl PinCapabilities {
    /// The pin's capabilities, as the pin capability parameter's answer.
    #[must_use]
    pub fn of(answer: u32) -> Self {
        let bit = |at: u32| answer & (1 << at) != 0;
        Self {
            impedance_sense: bit(0),
            trigger_required: bit(1),
            presence_detect: bit(2),
            headphone_drive: bit(3),
            output_capable: bit(4),
            input_capable: bit(5),
            balanced: bit(6),
            amplifier_capable: bit(16),
        }
    }
}

/// The device a pin belongs to, as its configuration default says.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Jack {
    /// A line-level output.
    LineOut,
    /// A speaker.
    Speaker,
    /// A headphone.
    Headphone,
    /// A compact disc input.
    Cd,
    /// A digital output.
    DigitalOut,
    /// A line-level input.
    LineIn,
    /// An auxiliary input.
    Auxiliary,
    /// A microphone.
    Microphone,
    /// A telephone.
    Telephone,
    /// Something else.
    Other,
}

impl Jack {
    /// The device, as the configuration default's device field carries it.
    #[must_use]
    pub fn of(kind: u32) -> Self {
        match kind & 0x0F {
            0x00 => Self::LineOut,
            0x01 => Self::Speaker,
            0x02 => Self::Headphone,
            0x03 => Self::Cd,
            0x04 | 0x05 => Self::DigitalOut,
            0x08 => Self::LineIn,
            0x09 => Self::Auxiliary,
            0x0A => Self::Microphone,
            0x0B => Self::Telephone,
            _ => Self::Other,
        }
    }

    /// Whether the device plays sound out of the pin.
    #[must_use]
    pub fn plays_out(&self) -> bool {
        matches!(
            self,
            Self::LineOut | Self::Speaker | Self::Headphone | Self::DigitalOut
        )
    }

    /// Whether the device takes sound in through the pin.
    #[must_use]
    pub fn takes_in(&self) -> bool {
        matches!(
            self,
            Self::LineIn | Self::Auxiliary | Self::Microphone | Self::Telephone | Self::Cd
        )
    }
}

/// A pin's configuration default: which device it belongs to and what it is
/// grouped with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ConfigDefault {
    /// The pin's sequence among its own.
    pub sequence: u8,
    /// The pin's association, 0 to 15.
    pub association: u8,
    /// The device the pin belongs to.
    pub device: Jack,
    /// Where the jack is, as the location field carries it.
    pub location: u8,
}

impl ConfigDefault {
    /// The default, as the configuration default verb's answer.
    #[must_use]
    pub fn of(answer: u32) -> Self {
        Self {
            sequence: (answer & 0x0F) as u8,
            association: ((answer >> 4) & 0x0F) as u8,
            device: Jack::of((answer >> 20) & 0x0F),
            location: ((answer >> 24) & 0x3F) as u8,
        }
    }
}

/// The most connections a node may have.
pub const MAX_CONNECTIONS: usize = 32;

/// A node's connection list: the nodes it is wired to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Connections {
    ids: [u8; MAX_CONNECTIONS],
    len: usize,
}

impl Default for Connections {
    fn default() -> Self {
        Self {
            ids: [0; MAX_CONNECTIONS],
            len: 0,
        }
    }
}

impl Connections {
    /// The connection list the codec answered with.
    ///
    /// The list is packed two connections to a byte, the first in the low
    /// nibble, and `count` says how many connections there are; a byte with
    /// its top bit set is the last one.
    #[must_use]
    pub fn unpack(bytes: &[u8], count: usize) -> Self {
        let mut out = Self::default();
        for byte in bytes.iter().copied() {
            if out.len >= count || out.len >= MAX_CONNECTIONS {
                break;
            }
            out.ids[out.len] = byte & 0x0F;
            out.len += 1;
            if byte & 0x80 != 0 {
                break;
            }
            if out.len >= count || out.len >= MAX_CONNECTIONS {
                break;
            }
            out.ids[out.len] = (byte >> 4) & 0x0F;
            out.len += 1;
        }
        out
    }

    /// The nodes this one is wired to.
    #[must_use]
    pub fn ids(&self) -> &[u8] {
        self.ids.get(..self.len).unwrap_or_default()
    }

    /// Whether `id` is among them.
    #[must_use]
    pub fn contains(&self, id: u8) -> bool {
        self.ids().contains(&id)
    }
}
