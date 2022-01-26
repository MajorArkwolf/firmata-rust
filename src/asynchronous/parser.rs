use byteorder::{ByteOrder, LittleEndian};
use bytes::BytesMut;

use crate::message::{
    get_header_type, Analog, AnalogMappingResponse, CapabilityResponse, Digital, Header, I2cReply,
    MessageIn, ReportFirmware, System,
};
use crate::protocol_constants::{
    ANALOG_MAPPING_RESPONSE, CAPABILITY_RESPONSE, I2C_MODE_READ, REPORT_FIRMWARE,
};
use crate::{FirmataError, PinId, Result};

fn parse_system_message(buf: &[u8]) -> Result<System> {
    match buf[0] {
        ANALOG_MAPPING_RESPONSE => {
            let message_out = AnalogMappingResponse::deserialize(&buf[1..]);
            Ok(System::AnalogMappingResponse(message_out))
        }

        CAPABILITY_RESPONSE => {
            let message_out = CapabilityResponse::deserialize(&buf[1..])?;
            Ok(System::CapabilityResponseMessage(message_out))
        }
        I2C_MODE_READ => {
            let message_out = I2cReply::deserialize(&buf[1..]);
            Ok(System::I2cReplyMessage(message_out))
        }
        REPORT_FIRMWARE => {
            let message_out = ReportFirmware::deserialize(&buf[1..])?;
            Ok(System::ReportFirmwareMessage(message_out))
        }
        _ => Err(FirmataError::ParseError(
            "did not find an expected system message",
            buf.to_vec(),
        )),
    }
}

pub fn parse_data(buf: &mut BytesMut) -> Result<MessageIn> {
    let header = get_header_type(buf[0])?;
    match header {
        // Prune the sysex messages out and pass in for deserialization
        Header::System => Ok(MessageIn::System(parse_system_message(
            &buf[1..buf.len() - 1],
        )?)),
        Header::AnalogMessage => {
            let value: u16 = LittleEndian::read_u16(&buf[1..3]);
            // Analog message can only do a range between 0..15, if you need to address
            // greater then 15 you need to use ANALOG_EXTENDED.
            let pin = buf[0] & 0x0F;
            let analog_message = Analog {
                pin: PinId::Analog(pin),
                value,
            };
            Ok(MessageIn::Analog(analog_message))
        }
        Header::DigitalMessage => {
            let port = buf[0] & 0x0F;
            let value: u16 = LittleEndian::read_u16(&buf[1..3]);
            let digital_message = Digital { port, value };
            Ok(MessageIn::Digital(digital_message))
        }
        Header::ProtocolVersion => {
            let protocol_version = format!("{:o}.{:o}", buf[0], buf[1]);
            Ok(MessageIn::ProtocolVersion(protocol_version))
        }
    }
}
