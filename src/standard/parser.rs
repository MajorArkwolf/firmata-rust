use crate::message::{get_header_type, Header};
use crate::message::{AnalogMappingResponse, CapabilityResponse, I2cReply, ReportFirmware};
use crate::protocol_constants::{
    ANALOG_MAPPING_RESPONSE, CAPABILITY_RESPONSE, END_SYSEX, I2C_MODE_READ, REPORT_FIRMWARE,
};
use crate::{message, FirmataError, PinId, Result};
use byteorder::{ByteOrder, LittleEndian};
use message::{Analog, Digital, Message, MessageId, MessageIn};

pub fn read_and_parse<T: std::io::Read>(
    reader: &mut T,
    timeout: std::time::Duration,
) -> Result<Message> {
    let start_of_header: &mut [u8; 1] = &mut [0; 1];
    let header_enum = loop {
        let start = std::time::Instant::now();
        let n = reader.read(start_of_header)?;
        if n == 1 {
            let header_result = get_header_type(start_of_header[0]);
            if header_result.is_ok() {
                break header_result?;
            }
        }
        if start.elapsed() > timeout {
            return Err(FirmataError::Timeout(format!("{:?}", start.elapsed())));
        }
    };

    match header_enum {
        Header::System => read_and_parse_system(reader),
        Header::AnalogMessage => read_and_parse_analog(reader, start_of_header[0]),
        Header::DigitalMessage => read_and_parse_digital(reader, start_of_header[0]),
        Header::ProtocolVersion => read_and_parse_protocol_version(reader),
    }
}

/// Firmata protocol uses the first byte to embed the pin id inside of a nibble, so we also need that
/// of information as well.
pub fn read_and_parse_analog<T: std::io::Read>(reader: &mut T, first_byte: u8) -> Result<Message> {
    let buf: &mut [u8; 2] = &mut [0; 2];
    reader.read_exact(buf)?;
    let value: u16 = LittleEndian::read_u16(buf);
    // Analog message can only do a range between 0..15, if you need to address
    // greater then 15 you need to use ANALOG_EXTENDED.
    let pin = first_byte & 0x0F;
    let analog_message = Analog {
        pin: PinId::Analog(pin),
        value,
    };
    Ok(Message {
        message_id: MessageId::Analog,
        message: MessageIn::Analog(analog_message),
    })
}

/// Firmata protocol uses the first byte to embed the pin id inside of a nibble, so we also need that
/// of information as well.
pub fn read_and_parse_digital<T: std::io::Read>(reader: &mut T, first_byte: u8) -> Result<Message> {
    let buf: &mut [u8; 2] = &mut [0; 2];
    reader.read_exact(buf)?;
    let port = first_byte & 0x0F;
    let value: u16 = LittleEndian::read_u16(buf);
    let digital_message = Digital { port, value };
    Ok(Message {
        message_id: MessageId::Digital,
        message: MessageIn::Digital(digital_message),
    })
}

pub fn read_and_parse_protocol_version<T: std::io::Read>(reader: &mut T) -> Result<Message> {
    let buf: &mut [u8; 2] = &mut [0; 2];
    reader.read_exact(buf)?;
    let protocol_version = format!("{:o}.{:o}", buf[0], buf[1]);
    Ok(Message {
        message_id: MessageId::ProtocolVersion,
        message: MessageIn::ProtocolVersion(protocol_version),
    })
}

pub fn read_and_parse_system<T: std::io::Read>(reader: &mut T) -> Result<Message> {
    let mut payload: Vec<u8> = vec![];
    let byte_in: &mut [u8; 1] = &mut [0; 1];
    // Read until we find our end of system flag or another header that should not of been there.
    loop {
        reader.read_exact(byte_in)?;
        if byte_in[0] == END_SYSEX {
            break;
        }
        // If get_header_type returns a valid header then there is message overlap and
        // this process should terminate
        else if get_header_type(byte_in[0]).is_ok() {
            return Err(FirmataError::ParseError(
                "found an unexpected message header when parsing a system message",
                payload,
            ));
        }
        payload.push(byte_in[0]);
    }

    // The first byte in the payload contains what message we expect.
    let byte = payload
        .get(0)
        .ok_or(FirmataError::OutOfRange("index out of range"))?;
    match *byte {
        ANALOG_MAPPING_RESPONSE => {
            let message_out = AnalogMappingResponse::deserialize(&payload[1..]);
            Ok(AnalogMappingResponse::into_message(message_out))
        }

        CAPABILITY_RESPONSE => {
            let message_out = CapabilityResponse::deserialize(&payload[1..])?;
            Ok(CapabilityResponse::into_message(message_out))
        }
        I2C_MODE_READ => {
            let message_out = I2cReply::deserialize(&payload[1..]);
            Ok(I2cReply::into_message(message_out))
        }
        REPORT_FIRMWARE => {
            let message_out = ReportFirmware::deserialize(&payload[1..])?;
            Ok(ReportFirmware::into_message(message_out))
        }
        _ => Err(FirmataError::ParseError(
            "did not find an expected system message",
            payload,
        )),
    }
}
