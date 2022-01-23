use super::protocol_constants::{
    is_id, ANALOG_MESSAGE, ANALOG_MESSAGE_END, DIGITAL_MESSAGE, DIGITAL_MESSAGE_END,
    PROTOCOL_VERSION, START_SYSEX,
};
use super::{FirmataError, I2CReply, Pin, PinId, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Header {
    System,
    AnalogMessage,
    DigitalMessage,
    ProtocolVersion,
}

/// Checks a byte to see if it contains a valid header byte.
/// # Errors
/// Returns [`FirmataError::ConversionFailure`] if no header byte
/// was found.
pub fn get_header_type(byte: u8) -> Result<Header> {
    if byte == PROTOCOL_VERSION {
        return Ok(Header::ProtocolVersion);
    } else if byte == START_SYSEX {
        return Ok(Header::System);
    } else if is_id(byte, ANALOG_MESSAGE..ANALOG_MESSAGE_END) {
        return Ok(Header::AnalogMessage);
    } else if is_id(byte, DIGITAL_MESSAGE..DIGITAL_MESSAGE_END) {
        return Ok(Header::DigitalMessage);
    }
    Err(FirmataError::ConversionFailure(
        "failed to convert u8 into message header",
    ))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageId {
    Analog = 1,
    Digital = 2,
    ProtocolVersion = 3,
    AnalogMapping = 4,
    Capability = 5,
    I2cReply = 6,
    ReportFirmware = 7,
}

#[derive(Debug, Clone)]
pub enum MessageIn {
    Analog(Analog),
    Digital(Digital),
    System(System),
    ProtocolVersion(String),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub message_id: MessageId,
    pub message: MessageIn,
}

#[derive(Debug, Clone, Copy)]
pub struct Analog {
    pub pin: PinId,
    pub value: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct Digital {
    pub port: u8,
    pub value: u16,
}

#[derive(Debug, Clone)]
pub enum System {
    AnalogMappingResponse(AnalogMappingResponse),
    CapabilityResponseMessage(CapabilityResponse),
    ReportFirmwareMessage(ReportFirmware),
    I2cReplyMessage(I2cReply),
}

#[derive(Debug, Clone)]
pub struct AnalogMappingResponse {
    pub supported_analog_pins: Vec<usize>,
}

impl AnalogMappingResponse {
    #[must_use]
    pub const fn into_message(message: Self) -> Message {
        Message {
            message_id: MessageId::AnalogMapping,
            message: MessageIn::System(System::AnalogMappingResponse(message)),
        }
    }

    #[must_use]
    pub fn deserialize(byte_stream: &[u8]) -> Self {
        let mut supported_analog_pins: Vec<usize> = vec![];
        for (index, value) in byte_stream.iter().enumerate() {
            if *value != 127_u8 {
                supported_analog_pins.push(index);
            }
        }

        Self {
            supported_analog_pins,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CapabilityResponse {
    pub pins: Vec<Pin>,
}

impl CapabilityResponse {
    #[must_use]
    pub const fn into_message(message: Self) -> Message {
        Message {
            message_id: MessageId::Capability,
            message: MessageIn::System(System::CapabilityResponseMessage(message)),
        }
    }

    /// # Errors
    /// Returns an out of bounds if the message parsed in is not valid.
    pub fn deserialize(byte_stream: &[u8]) -> Result<Self> {
        let mut a: Vec<Vec<u8>> = vec![];
        let x = byte_stream.iter().enumerate().filter(|x| *x.1 == 0x7F_u8);
        let mut last: usize = 0;
        for val in x {
            if val.0 != last {
                a.push(byte_stream[last..val.0].to_vec());
            } else {
                a.push(vec![]);
            }
            last = val.0 + 1;
        }
        let mut pins: Vec<Pin> = vec![];
        for pin_data in a {
            pins.push(Pin::deserialize(pin_data.as_slice())?);
        }
        Ok(Self { pins })
    }
}

#[derive(Debug, Clone)]
pub struct ReportFirmware {
    pub version: String,
    pub name: String,
}

impl ReportFirmware {
    #[must_use]
    pub const fn into_message(message: Self) -> Message {
        Message {
            message_id: MessageId::ReportFirmware,
            message: MessageIn::System(System::ReportFirmwareMessage(message)),
        }
    }

    /// # Errors
    /// Returns an out of bounds if the message parsed in is not valid.
    pub fn deserialize(byte_stream: &[u8]) -> Result<Self> {
        let version = format!("{:o}.{:o}", byte_stream[0], byte_stream[1]);
        let name = match String::from_utf8(byte_stream[2..byte_stream.len()].to_vec()) {
            Ok(v) => v.replace('\0', ""),
            Err(_) => {
                return Err(FirmataError::ParseError(
                    "failed to parse",
                    byte_stream.to_vec(),
                ))
            }
        };
        Ok(Self { version, name })
    }
}

#[derive(Debug, Clone)]
pub struct I2cReply {
    pub reply: I2CReply,
}

impl I2cReply {
    #[must_use]
    pub const fn into_message(message: Self) -> Message {
        Message {
            message_id: MessageId::I2cReply,
            message: MessageIn::System(System::I2cReplyMessage(message)),
        }
    }

    #[must_use]
    pub fn deserialize(byte_stream: &[u8]) -> Self {
        let len = byte_stream.len();
        let mut reply = I2CReply {
            address: i32::from(byte_stream[0]) | (i32::from(byte_stream[1]) << 7),
            register: i32::from(byte_stream[2]) | (i32::from(byte_stream[3]) << 7),
            data: vec![byte_stream[4] | byte_stream[5] << 7],
        };
        let mut i = 6;

        while i < len {
            if byte_stream[i] == 0xF7 {
                break;
            }
            if i + 2 > len {
                break;
            }
            reply.data.push(byte_stream[i] | byte_stream[i + 1] << 7);
            i += 2;
        }
        Self { reply }
    }
}
