//! The host tests: the bytes a dongle sends, and the bytes a driver has to
//! make of them.
//!
//! Every fixture here is a buffer the test writes itself, so a test that
//! passes says that the crate reads the specification's bytes the way the
//! specification says they are read.

use crate::cdc::{self, ClassRequest, LineCoding, Notification};
use crate::ecm;
use crate::ncm::{self, Ntb16, Ntb16Header, Ntb32, Ntb32Header};
use crate::rndis::{self, DataHeader, Init, MessageHeader, Query};

/// A 16-byte-header block with two four-byte datagrams in it.
fn ntb16_block() -> [u8; 48] {
    let mut out = [0_u8; 48];
    let header = Ntb16Header::new(7, 48, 20);
    out[0..12].copy_from_slice(&header.encode());
    out[12..16].copy_from_slice(&[0xAA, 0xAA, 0xAA, 0xAA]);
    out[16..20].copy_from_slice(&[0xBB, 0xBB, 0xBB, 0xBB]);
    out[20..24].copy_from_slice(&ncm::NDP16_SIGNATURE_0.to_le_bytes());
    out[24..26].copy_from_slice(&0_u16.to_le_bytes());
    out[26..28].copy_from_slice(&16_u16.to_le_bytes());
    out[28..30].copy_from_slice(&12_u16.to_le_bytes());
    out[30..32].copy_from_slice(&4_u16.to_le_bytes());
    out[32..34].copy_from_slice(&16_u16.to_le_bytes());
    out[34..36].copy_from_slice(&4_u16.to_le_bytes());
    out
}

/// A 32-byte-header block with two four-byte datagrams in it.
fn ntb32_block() -> [u8; 56] {
    let mut out = [0_u8; 56];
    let header = Ntb32Header::new(9, 56, 24);
    out[0..16].copy_from_slice(&header.encode());
    out[16..20].copy_from_slice(&[0xCC, 0xCC, 0xCC, 0xCC]);
    out[20..24].copy_from_slice(&[0xDD, 0xDD, 0xDD, 0xDD]);
    out[24..28].copy_from_slice(&ncm::NDP32_SIGNATURE_0.to_le_bytes());
    out[28..30].copy_from_slice(&0_u16.to_le_bytes());
    out[30..32].copy_from_slice(&0_u16.to_le_bytes());
    out[32..36].copy_from_slice(&32_u32.to_le_bytes());
    out[36..40].copy_from_slice(&0_u32.to_le_bytes());
    out[40..44].copy_from_slice(&16_u32.to_le_bytes());
    out[44..48].copy_from_slice(&4_u32.to_le_bytes());
    out[48..52].copy_from_slice(&20_u32.to_le_bytes());
    out[52..56].copy_from_slice(&4_u32.to_le_bytes());
    out
}

#[test]
fn a_class_request_carries_its_parts() {
    let request = cdc::set_packet_filter(0x1F, 2);
    let bytes = request.bytes();
    assert_eq!(bytes[0], 0x21, "the class request type");
    assert_eq!(bytes[1], 0x43, "SET_ETHERNET_PACKET_FILTER");
    assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 0x1F, "the filter bits");
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 2, "the interface");
    assert_eq!(
        u16::from_le_bytes([bytes[6], bytes[7]]),
        0,
        "no data stage"
    );
}

#[test]
fn a_class_request_round_trips() {
    let request = ClassRequest::new(cdc::request::SET_NTB_FORMAT, 1, 3, 16);
    let decoded = ClassRequest::decode(&request.bytes());
    assert_eq!(decoded, Some(request), "the request the bytes carry");
}

#[test]
fn a_vendor_request_is_not_a_class_request() {
    let mut bytes = cdc::set_packet_filter(0x1F, 0).bytes();
    bytes[0] = 0x40;
    assert_eq!(ClassRequest::decode(&bytes), None, "a vendor request");
}

#[test]
fn a_short_setup_packet_is_not_a_request() {
    assert_eq!(
        ClassRequest::decode(&[0x21, 0x43, 0x00]),
        None,
        "three bytes are not a setup packet"
    );
}

#[test]
fn the_class_request_codes_are_the_specification_s_own() {
    assert_eq!(cdc::request::SEND_ENCAPSULATED_COMMAND, 0x00, "SEND");
    assert_eq!(cdc::request::GET_ENCAPSULATED_RESPONSE, 0x01, "GET");
    assert_eq!(cdc::request::SET_LINE_CODING, 0x20, "SET_LINE_CODING");
    assert_eq!(cdc::request::GET_LINE_CODING, 0x21, "GET_LINE_CODING");
    assert_eq!(cdc::request::SET_CONTROL_LINE_STATE, 0x22, "SET_CONTROL");
    assert_eq!(cdc::request::SET_ETHERNET_MULTICAST_FILTERS, 0x40, "MULTICAST");
    assert_eq!(cdc::request::SET_ETHERNET_PACKET_FILTER, 0x43, "PACKET_FILTER");
    assert_eq!(cdc::request::GET_NTB_PARAMETERS, 0x80, "GET_NTB_PARAMETERS");
    assert_eq!(cdc::request::SET_NTB_FORMAT, 0x84, "SET_NTB_FORMAT");
    assert_eq!(cdc::request::SET_NTB_INPUT_SIZE, 0x86, "SET_NTB_INPUT_SIZE");
    assert_eq!(cdc::request::SET_MAX_DATAGRAM_SIZE, 0x88, "SET_MAX_DATAGRAM");
    assert_eq!(cdc::request::SET_CRC_MODE, 0x8A, "SET_CRC_MODE");
}

#[test]
fn the_subclasses_are_the_specification_s_own() {
    assert_eq!(cdc::subclass::ETHERNET, 0x06, "ETHERNET");
    assert_eq!(cdc::subclass::EEM, 0x0C, "EEM");
    assert_eq!(cdc::subclass::NCM, 0x0D, "NCM");
    assert_eq!(cdc::subclass::MBIM, 0x0E, "MBIM");
}

#[test]
fn the_packet_filter_bits_are_the_specification_s_own() {
    assert_eq!(cdc::packet_filter::PROMISCUOUS, 1 << 0, "promiscuous");
    assert_eq!(cdc::packet_filter::ALL_MULTICAST, 1 << 1, "all multicast");
    assert_eq!(cdc::packet_filter::DIRECTED, 1 << 2, "directed");
    assert_eq!(cdc::packet_filter::BROADCAST, 1 << 3, "broadcast");
    assert_eq!(cdc::packet_filter::MULTICAST, 1 << 4, "multicast");
}

#[test]
fn the_control_line_state_bits_are_the_specification_s_own() {
    assert_eq!(cdc::control_line_state::DTR, 1 << 0, "DTR");
    assert_eq!(cdc::control_line_state::RTS, 1 << 1, "RTS");
}

#[test]
fn a_notification_round_trips() {
    let notification = Notification {
        request_type: 0x82,
        request: cdc::notification::NETWORK_CONNECTION,
        value: 1,
        index: 0,
        length: 0,
    };
    let bytes = notification.encode();
    assert_eq!(bytes.len(), 8, "a notification is eight bytes");
    assert_eq!(Notification::decode(&bytes), Some(notification), "the notification");
}

#[test]
fn a_short_notification_is_not_a_notification() {
    assert_eq!(
        Notification::decode(&[0x82, 0x00, 0x00]),
        None,
        "three bytes are not a notification"
    );
}

#[test]
fn a_notification_says_what_it_is_about() {
    let connected = Notification {
        request_type: 0x82,
        request: cdc::notification::NETWORK_CONNECTION,
        value: 1,
        index: 0,
        length: 0,
    };
    let response = Notification {
        request_type: 0x82,
        request: cdc::notification::RESPONSE_AVAILABLE,
        value: 0,
        index: 0,
        length: 0,
    };
    assert!(connected.is_connection_change(), "the link came up");
    assert!(!connected.is_response_available(), "not a response");
    assert!(response.is_response_available(), "an answer is waiting");
}

#[test]
fn a_line_coding_round_trips() {
    let coding = LineCoding {
        dte_rate: 115_200,
        char_format: 0,
        parity_type: 0,
        data_bits: 8,
    };
    let bytes = coding.encode();
    assert_eq!(bytes.len(), 7, "a line coding is seven bytes");
    assert_eq!(LineCoding::decode(&bytes), Some(coding), "the line coding");
}

#[test]
fn a_short_line_coding_is_not_a_line_coding() {
    assert_eq!(
        LineCoding::decode(&[0x00, 0x00, 0x00, 0x00]),
        None,
        "four bytes are not a line coding"
    );
}

#[test]
fn the_multicast_filter_request_carries_the_bitmap_length() {
    let bitmap = [0_u8; ecm::MULTICAST_FILTER_BYTES];
    let request = cdc::set_ethernet_multicast_filters(&bitmap, 4);
    assert_eq!(
        request.expect("the bitmap fits a length").length,
        128,
        "the bitmap's length"
    );
    assert_eq!(request.expect("the bitmap fits a length").request, 0x40, "the code");
}

#[test]
fn an_encapsulated_command_carries_the_message_length() {
    let request = cdc::send_encapsulated_command(&[0_u8; 28], 1);
    assert_eq!(request.expect("the message fits").length, 28, "the length");
    assert_eq!(request.expect("the message fits").request, 0x00, "the code");
}

#[test]
fn an_ecm_setup_sends_its_three_requests_in_order() {
    let setup = ecm::Setup::standard();
    let requests = setup.requests(2);
    assert_eq!(requests[0].request, 0x43, "the packet filter first");
    assert_eq!(requests[1].request, 0x40, "then the multicast bitmap");
    assert_eq!(requests[2].request, 0x22, "then the control line state");
    assert_eq!(requests[0].value, 0x1F, "every frame the station may see");
    assert_eq!(requests[2].value, 0x03, "DTR and RTS");
}

#[test]
fn an_ecm_setup_keeps_its_multicast_bitmap() {
    let mut setup = ecm::Setup::standard();
    setup.multicast_filters[0] = 0xA5;
    let bitmap = setup.multicast_filter_bytes();
    assert_eq!(bitmap[0], 0xA5, "the first filter byte");
    assert_eq!(bitmap.len(), 128, "the bitmap's length");
}

#[test]
fn an_ecm_transfer_is_one_frame() {
    let frame = [0_u8; 64];
    assert_eq!(
        ecm::frame_from_transfer(&frame).expect("a frame"),
        &frame[..],
        "the transfer is the frame"
    );
    assert_eq!(
        ecm::frame_from_transfer(&[0_u8; 13]),
        None,
        "a transfer too short for a header is not a frame"
    );
}

#[test]
fn an_ecm_frame_carries_its_type() {
    let mut frame = [0_u8; 64];
    frame[12] = 0x08;
    frame[13] = 0x00;
    assert_eq!(ecm::frame_ethertype(&frame), Some(0x0800), "the type field");
    assert_eq!(ecm::frame_ethertype(&frame[..10]), None, "a short frame");
}

#[test]
fn an_ecm_descriptor_carries_the_station_s_address() {
    let mut descriptor = [0_u8; 13];
    descriptor[0] = 13;
    descriptor[1] = ecm::ETHERNET_DESCRIPTOR_TYPE;
    descriptor[2] = 0x02;
    descriptor[3] = 0x11;
    descriptor[4] = 0x22;
    descriptor[5] = 0x33;
    descriptor[6] = 0x44;
    descriptor[7] = 0x55;
    assert_eq!(
        ecm::mac_from_descriptor(&descriptor),
        Some([0x02, 0x11, 0x22, 0x33, 0x44, 0x55]),
        "the station's address"
    );
    descriptor[1] = 0x05;
    assert_eq!(
        ecm::mac_from_descriptor(&descriptor),
        None,
        "another descriptor type carries no address"
    );
}

#[test]
fn an_ntb16_header_round_trips() {
    let header = Ntb16Header::new(3, 512, 480);
    let bytes = header.encode();
    assert_eq!(bytes.len(), 12, "a 16-byte header is twelve bytes");
    assert_eq!(Ntb16Header::decode(&bytes), Some(header), "the header");
}

#[test]
fn an_ntb32_header_is_not_an_ntb16_header() {
    let header = Ntb32Header::new(3, 512, 480);
    assert_eq!(
        Ntb16Header::decode(&header.encode()),
        None,
        "a 32-byte header has another signature"
    );
}

#[test]
fn an_ntb16_header_of_another_signature_is_refused() {
    let mut header = Ntb16Header::new(3, 512, 480);
    header.signature = 0xDEAD_BEEF;
    assert_eq!(
        Ntb16Header::decode(&header.encode()),
        None,
        "another signature is not a block"
    );
}

#[test]
fn an_ntb32_header_round_trips() {
    let header = Ntb32Header::new(11, 4096, 4000);
    let bytes = header.encode();
    assert_eq!(bytes.len(), 16, "a 32-byte header is sixteen bytes");
    assert_eq!(Ntb32Header::decode(&bytes), Some(header), "the header");
}

#[test]
fn an_ntb16_block_names_its_datagrams() {
    let block = ntb16_block();
    let parsed = Ntb16::parse(&block).expect("a well-formed block");
    assert_eq!(parsed.datagram_count, 2, "two datagrams");
    assert_eq!(parsed.header.sequence_number, 7, "the block's number");
    assert_eq!(
        parsed.datagram_bytes(&block, 0).expect("the first datagram"),
        &[0xAA, 0xAA, 0xAA, 0xAA],
        "the first datagram"
    );
    assert_eq!(
        parsed.datagram_bytes(&block, 1).expect("the second datagram"),
        &[0xBB, 0xBB, 0xBB, 0xBB],
        "the second datagram"
    );
}

#[test]
fn an_ntb16_block_of_a_truncated_pointer_list_is_refused() {
    let mut block = ntb16_block();
    block[26..28].copy_from_slice(&8_u16.to_le_bytes());
    assert_eq!(Ntb16::parse(&block), None, "a pointer list below the minimum");
}

#[test]
fn an_ntb16_block_with_a_wrong_pointer_signature_is_refused() {
    let mut block = ntb16_block();
    block[20..24].copy_from_slice(&0x1234_5678_u32.to_le_bytes());
    assert_eq!(Ntb16::parse(&block), None, "another signature is not a list");
}

#[test]
fn an_ntb16_block_that_lies_about_its_length_is_refused() {
    let mut block = ntb16_block();
    block[8..10].copy_from_slice(&900_u16.to_le_bytes());
    assert_eq!(Ntb16::parse(&block), None, "a block longer than the buffer");
}

#[test]
fn an_ntb32_block_names_its_datagrams() {
    let block = ntb32_block();
    let parsed = Ntb32::parse(&block).expect("a well-formed block");
    assert_eq!(parsed.datagram_count, 2, "two datagrams");
    assert_eq!(
        parsed.datagram_bytes(&block, 1).expect("the second datagram"),
        &[0xDD, 0xDD, 0xDD, 0xDD],
        "the second datagram"
    );
}

#[test]
fn a_datagram_is_found_in_either_version_of_a_block() {
    let sixteen = ntb16_block();
    let thirty_two = ntb32_block();
    assert_eq!(
        ncm::find_datagram(&sixteen, 0).expect("the datagram"),
        &[0xAA, 0xAA, 0xAA, 0xAA],
        "the 16-byte block's first datagram"
    );
    assert_eq!(
        ncm::find_datagram(&thirty_two, 0).expect("the datagram"),
        &[0xCC, 0xCC, 0xCC, 0xCC],
        "the 32-byte block's first datagram"
    );
    assert_eq!(
        ncm::find_datagram(&sixteen, 9),
        None,
        "a datagram the list does not name"
    );
}

#[test]
fn a_block_says_which_version_it_is() {
    assert_eq!(
        ncm::version(&ntb16_block()),
        Some(ncm::NtbVersion::Sixteen),
        "the 16-byte block"
    );
    assert_eq!(
        ncm::version(&ntb32_block()),
        Some(ncm::NtbVersion::ThirtyTwo),
        "the 32-byte block"
    );
    assert_eq!(ncm::version(&[0_u8; 32]), None, "neither");
}

#[test]
fn an_ncm_message_header_round_trips() {
    let header = ncm::NcmMessageHeader {
        message_type: 1,
        header_length: 16,
        sequence_number: 4,
        message_length: 64,
        ndp_index: 48,
        reserved: 0,
    };
    let bytes = header.encode();
    assert_eq!(bytes.len(), 16, "a message header is sixteen bytes");
    assert_eq!(ncm::NcmMessageHeader::decode(&bytes), Some(header), "the header");
}

#[test]
fn a_block_behind_a_message_header_is_found() {
    let mut transfer = [0_u8; 64];
    transfer[16..28].copy_from_slice(&ntb16_block()[0..12]);
    assert_eq!(
        ncm::block_after_message_header(&transfer).expect("the block"),
        &transfer[16..],
        "the block behind the header"
    );
    assert_eq!(
        ncm::block_after_message_header(&ntb16_block()),
        None,
        "a block with no message header in front"
    );
}

#[test]
fn the_ncm_setup_requests_are_the_specification_s_own() {
    let parameters = ncm::get_ntb_parameters(1);
    assert_eq!(parameters.request, 0x80, "GET_NTB_PARAMETERS");
    let format = ncm::set_ntb_format(ncm::NTB32_FORMAT, 1);
    assert_eq!(format.request, 0x84, "SET_NTB_FORMAT");
    assert_eq!(format.value, 1, "the 32-byte format");
    let size = ncm::set_ntb_input_size(4096, 1);
    assert_eq!(size.request, 0x86, "SET_NTB_INPUT_SIZE");
    assert_eq!(size.value, 4096, "the input size");
}

#[test]
fn a_rndis_message_header_round_trips() {
    let header = MessageHeader {
        message_type: rndis::message_type::INIT | rndis::message_type::COMPLETION_FLAG,
        message_length: 28,
        request_id: 5,
        status: rndis::status::SUCCESS,
    };
    let bytes = header.encode();
    assert_eq!(bytes.len(), 16, "a message header is sixteen bytes");
    assert_eq!(MessageHeader::decode(&bytes), Some(header), "the header");
}

#[test]
fn a_rndis_message_header_of_another_type_is_refused() {
    assert_eq!(
        MessageHeader::decode(&[0_u8; 12]),
        None,
        "twelve bytes are not a message header"
    );
}

#[test]
fn an_rndis_init_message_carries_the_version_and_the_size() {
    let init = Init::new(1, 0, 0x4000);
    let bytes = init.message(3);
    assert_eq!(bytes.len(), 28, "an initialization message is 28 bytes");
    assert_eq!(
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        rndis::message_type::INIT,
        "the message type"
    );
    assert_eq!(
        u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        3,
        "the request number"
    );
    assert_eq!(
        u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
        1,
        "the major version"
    );
    assert_eq!(
        u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
        0x4000,
        "the largest message the driver will send"
    );
}

#[test]
fn an_rndis_query_message_carries_the_identifier() {
    let query = Query::new(rndis::oid::GEN_MEDIA_CONNECT_STATUS);
    let bytes = query.message(9);
    assert_eq!(bytes.len(), 28, "a query message is 28 bytes");
    assert_eq!(
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        rndis::message_type::QUERY,
        "the message type"
    );
    assert_eq!(
        u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
        0x0001_0114,
        "the object identifier"
    );
    assert_eq!(
        u16::from_le_bytes([bytes[20], bytes[21]]),
        0,
        "no information buffer"
    );
}

#[test]
fn an_rndis_set_message_is_a_set() {
    let query = Query::new(rndis::oid::CURRENT_ADDRESS);
    let bytes = query.set_message(10);
    assert_eq!(
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        rndis::message_type::SET,
        "the message type"
    );
}

#[test]
fn the_rndis_identifiers_are_the_specification_s_own() {
    assert_eq!(rndis::oid::GEN_MEDIA_CONNECT_STATUS, 0x0001_0114, "the link state");
    assert_eq!(rndis::oid::PERMANENT_ADDRESS, 0x0101_0101, "the permanent address");
    assert_eq!(rndis::oid::CURRENT_ADDRESS, 0x0101_0102, "the current address");
}

#[test]
fn the_rndis_statuses_are_the_specification_s_own() {
    assert_eq!(rndis::status::SUCCESS, 0, "success");
    assert_eq!(rndis::status::FAILURE, 0xC000_0001, "failure");
    assert_eq!(rndis::status::INVALID_DATA, 0xC001_0015, "invalid data");
    assert_eq!(rndis::status::NOT_SUPPORTED, 0xC000_00BB, "not supported");
    assert_eq!(rndis::status::MEDIA_CONNECT, 0x4001_000B, "the link is up");
}

#[test]
fn an_rndis_data_header_round_trips() {
    let header = DataHeader {
        message_type: rndis::message_type::PACKET,
        message_length: 100,
        request_id: 0,
        status: rndis::status::SUCCESS,
        offset: u32::try_from(rndis::DATA_OFFSET).expect("the offset fits"),
        length: 60,
        data_length: 60,
        reserved: 0,
    };
    let bytes = header.encode();
    assert_eq!(bytes.len(), 36, "a data header is 36 bytes");
    assert_eq!(DataHeader::decode(&bytes), Some(header), "the header");
}

#[test]
fn an_rndis_data_header_finds_its_payload() {
    let header = DataHeader {
        message_type: rndis::message_type::PACKET,
        message_length: 40,
        request_id: 0,
        status: rndis::status::SUCCESS,
        offset: u32::try_from(rndis::DATA_OFFSET).expect("the offset fits"),
        length: 4,
        data_length: 4,
        reserved: 0,
    };
    let mut message = [0_u8; 40];
    message[0..36].copy_from_slice(&header.encode());
    message[36..40].copy_from_slice(&[1, 2, 3, 4]);
    assert_eq!(
        header.payload(&message).expect("the payload"),
        &[1, 2, 3, 4],
        "the frame the device sent"
    );
}

#[test]
fn an_rndis_device_has_states() {
    use rndis::{Command, State};
    assert_eq!(
        State::Uninitialized.next(Command::Init),
        Some(State::Initialized),
        "the initialization puts the device in the second state"
    );
    assert_eq!(
        State::Uninitialized.next(Command::Query),
        None,
        "a device that is not initialized cannot be queried"
    );
    assert_eq!(
        State::Initialized.next(Command::Packet),
        Some(State::DataCarrying),
        "the first frame puts the device in the third state"
    );
    assert_eq!(
        State::Initialized.next(Command::Halt),
        Some(State::Uninitialized),
        "the halt puts the device back"
    );
    assert_eq!(
        State::DataCarrying.next(Command::Packet),
        Some(State::DataCarrying),
        "a device that carries frames keeps carrying them"
    );
}

#[test]
fn an_rndis_reply_is_matched_by_its_request_number() {
    let reply = MessageHeader {
        message_type: rndis::message_type::INIT | rndis::message_type::COMPLETION_FLAG,
        message_length: 28,
        request_id: 5,
        status: rndis::status::SUCCESS,
    };
    let other = MessageHeader {
        request_id: 6,
        ..reply
    };
    assert!(reply.is_reply_to(5), "the reply to the request");
    assert!(!reply.is_reply_to(6), "not a reply to another request");
    assert!(!other.is_reply_to(5), "the other request's reply");
    assert!(reply.is_completion(), "a reply carries the completion bit");
    assert_eq!(reply.base_type(), rndis::message_type::INIT, "the message type");
}

#[test]
fn the_reply_to_a_request_is_found_among_the_replies() {
    let first = MessageHeader {
        message_type: rndis::message_type::QUERY | rndis::message_type::COMPLETION_FLAG,
        message_length: 28,
        request_id: 1,
        status: rndis::status::SUCCESS,
    };
    let second = MessageHeader {
        request_id: 2,
        ..first
    };
    let replies = [first, second];
    assert_eq!(
        rndis::find_reply(&replies, 2).expect("the reply").request_id,
        2,
        "the reply to the second request"
    );
    assert_eq!(rndis::find_reply(&replies, 9), None, "no reply to that one");
}

#[test]
fn an_rndis_message_is_sent_as_an_encapsulated_command() {
    let init = Init::new(1, 0, 0x4000);
    let message = init.message(1);
    let request = rndis::send_message(&message, 2).expect("the message fits");
    assert_eq!(request.request, 0x00, "SEND_ENCAPSULATED_COMMAND");
    assert_eq!(request.length, 28, "the message's length");
}

#[test]
fn an_rndis_halt_message_is_a_halt() {
    let bytes = rndis::halt_message(4);
    assert_eq!(
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        rndis::message_type::HALT,
        "the message type"
    );
    assert_eq!(
        u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        4,
        "the request number"
    );
}
