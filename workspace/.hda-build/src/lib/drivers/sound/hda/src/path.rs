//! Which path a device's sound takes through the codec.
//!
//! A codec is a graph: a digital converter plays into an amplifier, which
//! plays into a mixer, which plays into the pin a jack is wired to. A
//! microphone's sound runs the other way. The driver has to find that path
//! before it can program anything, because every widget on it has to be
//! woken, connected and given its gain.
//!
//! This module finds the path and says which widget is which; it does not
//! program them, which is [`crate::codec`]'s verbs' work.

use crate::codec::{ConfigDefault, Connections, Jack, NodeType, PinCapabilities};

/// The codec's nodes, as the driver read them.
pub trait Nodes {
    /// How many nodes the codec has.
    fn count(&self) -> u8;
    /// What the node `id` is.
    fn kind(&self, id: u8) -> NodeType;
    /// The nodes the node `id` is wired to.
    fn connections(&self, id: u8) -> Connections;
}

/// Whether a pin can serve `device`.
///
/// A pin serves a device when the codec says the pin belongs to it and the
/// pin can do what the device needs: an output device needs a pin that
/// plays sound out, an input device one that takes sound in.
#[must_use]
pub fn serves(capabilities: &PinCapabilities, default: &ConfigDefault, device: Jack) -> bool {
    if default.device != device {
        return false;
    }
    if device.plays_out() {
        return capabilities.output_capable;
    }
    if device.takes_in() {
        return capabilities.input_capable;
    }
    false
}

/// The widgets between a stream's converter and a pin.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Path {
    /// The digital converter the stream plays from, or records into.
    pub converter: u8,
    /// The amplifier on the way, if there is one.
    pub amplifier: Option<u8>,
    /// The mixer on the way, if there is one.
    pub mixer: Option<u8>,
    /// The pin the sound leaves or arrives at.
    pub pin: u8,
}

/// The most nodes a path may have, the codec's node count.
pub const MAX_NODES: usize = 64;

/// The path from a converter to the pin `pin`, if there is one.
///
/// The search walks the codec backwards from the pin: the widget whose
/// connection list holds the pin is the one before it, and so on until a
/// converter is reached. A codec whose widgets are not wired in a line has
/// no path this finds.
#[must_use]
pub fn path(nodes: &impl Nodes, pin: u8) -> Option<Path> {
    let mut seen = [0_u8; MAX_NODES];
    let mut count = 0_usize;
    let mut current = pin;
    let mut amplifier = None;
    let mut mixer = None;
    for _ in 0..MAX_NODES {
        let Some(id) = upstream(nodes, current, &seen[..count]) else {
            return None;
        };
        if let Some(slot) = seen.get_mut(count) {
            *slot = id;
        }
        count += 1;
        match nodes.kind(id) {
            NodeType::Output | NodeType::Input => {
                return Some(Path {
                    converter: id,
                    amplifier,
                    mixer,
                    pin,
                });
            }
            NodeType::Mixer | NodeType::Selector => {
                if mixer.is_none() {
                    mixer = Some(id);
                }
            }
            NodeType::VolumeKnob => {
                if amplifier.is_none() {
                    amplifier = Some(id);
                }
            }
            _ => {}
        }
        current = id;
    }
    None
}

/// The node that is wired to `node`, if there is one that has not been seen.
fn upstream(nodes: &impl Nodes, node: u8, seen: &[u8]) -> Option<u8> {
    for id in 0..nodes.count() {
        if seen.contains(&id) {
            continue;
        }
        if nodes.connections(id).contains(node) {
            return Some(id);
        }
    }
    None
}
