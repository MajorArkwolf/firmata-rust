use super::parser;
use crate::protocol_constants::{
    ANALOG_MAPPING_QUERY, ANALOG_MESSAGE, CAPABILITY_QUERY, DIGITAL_MESSAGE, END_SYSEX, I2C_CONFIG,
    I2C_MODE_READ, I2C_MODE_WRITE, I2C_REQUEST, PIN_MODE, REPORT_ANALOG, REPORT_DIGITAL,
    REPORT_FIRMWARE, SAMPLEING_INTERVAL, START_SYSEX, STRING_DATA,
};
use crate::{message, FirmataError, I2CReply, Pin, PinId, PinMode, PinStates, Result};
use byteorder::LittleEndian;
use byteorder::WriteBytesExt;
use message::MessageId;
use message::MessageIn;
use serde::{Deserialize, Serialize};
use std::io;
use std::str;

/// A structure representing a firmata board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board<T: io::Read + io::Write> {
    connection: T,
    pin_state: PinStates,
    i2c_data: Vec<I2CReply>,
    protocol_version: String,
    firmware_name: String,
    firmware_version: String,
}

impl<T: io::Read + io::Write> Board<T> {
    /// Creates a new [`Board`] given an [`std::io::Read`] + [`std::io::Write`].
    pub fn new(connection: T) -> Self {
        Self {
            connection,
            firmware_name: String::new(),
            firmware_version: String::new(),
            protocol_version: String::new(),
            pin_state: PinStates::create(vec![]),
            i2c_data: vec![],
        }
    }

    /// Populates all the information of a given board
    /// # Errors
    /// This can return several firmata errors depending if its network, parsing
    /// or incorrect information.
    pub fn populate_board_info(&mut self) -> Result<()> {
        self.query_firmware()?;
        self.read_until_message_found(MessageId::ReportFirmware)?;
        self.query_capabilities()?;
        self.read_until_message_found(MessageId::Capability)?;
        self.query_analog_mapping()?;
        self.read_until_message_found(MessageId::AnalogMapping)?;
        Ok(())
    }

    pub fn pin_id_to_pin(&self, pin_in: PinId) -> u8 {
        match pin_in {
            PinId::Analog(x) => x + self.pin_state.analog_pin_start,
            PinId::Digital(y) => y,
            PinId::Pin(z) => z,
        }
    }

    pub fn get_physical_pin(&self, pin_in: PinId) -> Pin {
        let pin: usize = self.pin_id_to_pin(pin_in).into();
        self.pin_state.pins[pin].clone()
    }

    fn handle_message(&mut self, message: MessageIn) -> Result<()> {
        match message {
            message::MessageIn::Analog(v) => {
                if !self.pin_state.pins.is_empty() {
                    let pin: usize = self.pin_id_to_pin(v.pin).into();
                    if self.pin_state.pins[pin].analog {
                        self.pin_state.pins[pin].value = v.value;
                        return Ok(());
                    }
                }
                Err(FirmataError::UninitializedError(
                    "analog message arrived but the pins were not initialised",
                ))
            }
            message::MessageIn::Digital(v) => {
                if !self.pin_state.pins.is_empty() {
                    for i in 0..8 {
                        let pin = (8 * v.port) + i;

                        if self.pin_state.pins.len() > pin as usize
                            && self.pin_state.pins[pin as usize].mode == PinMode::Input
                        {
                            self.pin_state.pins[pin as usize].value =
                                (v.value >> (i & 0x07)) & 0x01;
                        }
                    }
                    return Ok(());
                }
                Err(FirmataError::UninitializedError(
                    "digital message arrived but the pins were not initialised",
                ))
            }
            message::MessageIn::System(v) => match v {
                message::System::AnalogMappingResponse(v) => {
                    if !self.pin_state.pins.is_empty() {
                        for id in v.supported_analog_pins {
                            self.pin_state.pins[id].analog = true;
                        }
                        return Ok(());
                    }
                    Err(FirmataError::UninitializedError(
                        "pins had not been initialised prior to mapping analog pins",
                    ))
                }
                message::System::CapabilityResponseMessage(v) => {
                    self.pin_state.pins = v.pins;
                    Ok(())
                }
                message::System::ReportFirmwareMessage(v) => {
                    self.firmware_name = v.name;
                    self.firmware_version = v.version;
                    Ok(())
                }
                message::System::I2cReplyMessage(v) => {
                    self.i2c_data.push(v.reply);
                    Ok(())
                }
            },
            message::MessageIn::ProtocolVersion(v) => {
                self.protocol_version = v;
                Ok(())
            }
        }
    }

    fn read_until_message_found(&mut self, message_id: MessageId) -> Result<()> {
        loop {
            match self.read(std::time::Duration::from_millis(0)) {
                Ok(v) => {
                    if v == message_id {
                        return Ok(());
                    }
                }
                Err(err) => match err {
                    FirmataError::UninitializedError(_) | FirmataError::NotFoundError(_) => {
                        continue
                    }
                    v => {
                        return Err(v);
                    }
                },
            }
        }
    }
}

impl<T: io::Read + io::Write> Board<T> {
    pub fn i2c_data(&mut self) -> &mut Vec<I2CReply> {
        &mut self.i2c_data
    }
    pub fn pins(&self) -> Vec<Pin> {
        self.pin_state.pins.clone()
    }

    pub fn protocol_version(&self) -> &str {
        &self.protocol_version
    }
    pub fn firmware_name(&self) -> &str {
        &self.firmware_name
    }
    pub fn firmware_version(&self) -> &str {
        &self.firmware_version
    }
    pub fn query_analog_mapping(&mut self) -> Result<()> {
        self.connection
            .write_all(&[START_SYSEX, ANALOG_MAPPING_QUERY, END_SYSEX])?;
        Ok(())
    }
    pub fn query_capabilities(&mut self) -> Result<()> {
        self.connection
            .write_all(&[START_SYSEX, CAPABILITY_QUERY, END_SYSEX])?;
        Ok(())
    }
    pub fn query_firmware(&mut self) -> Result<()> {
        self.connection
            .write_all(&[START_SYSEX, REPORT_FIRMWARE, END_SYSEX])?;
        Ok(())
    }

    pub fn i2c_config(&mut self, delay: u16) -> Result<()> {
        let bytes_out = delay.to_le_bytes();
        self.connection.write_all(&[
            START_SYSEX,
            I2C_CONFIG,
            bytes_out[0],
            bytes_out[1],
            END_SYSEX,
        ])?;
        Ok(())
    }

    pub fn i2c_read(&mut self, addr: u8, size: u16) -> Result<()> {
        let bytes_out = size.to_le_bytes();
        self.connection.write_all(&[
            START_SYSEX,
            I2C_REQUEST,
            addr,
            (I2C_MODE_READ << 3),
            bytes_out[0],
            bytes_out[1],
            END_SYSEX,
        ])?;
        Ok(())
    }

    pub fn i2c_write(&mut self, addr: u8, data: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(4 + data.len() * 2 + 1);

        buf.push(START_SYSEX);
        buf.push(I2C_REQUEST);
        buf.push(addr);
        buf.push(I2C_MODE_WRITE << 3);

        for i in data.iter() {
            buf.push(i & 0x7F);
            buf.push(((i32::from(*i) >> 7) & 0x7F) as u8);
        }
        buf.push(END_SYSEX);
        self.connection.write_all(&buf[..])?;
        Ok(())
    }

    pub fn report_digital(&mut self, pin: PinId, state: bool) -> Result<()> {
        let pin_out = match pin {
            PinId::Analog(_) => {
                return Err(FirmataError::WrongType(
                    "found analog pin expected analog pin",
                ))
            }
            PinId::Digital(v) | PinId::Pin(v) => v,
        };
        self.connection
            .write_all(&[REPORT_DIGITAL | pin_out, state as u8])?;
        Ok(())
    }

    pub fn report_analog(&mut self, pin: PinId, state: bool) -> Result<()> {
        let pin_out = match pin {
            PinId::Analog(_) => self.pin_id_to_pin(pin),
            PinId::Digital(_) => {
                return Err(FirmataError::WrongType(
                    "found digital pin expected analog pin",
                ))
            }
            PinId::Pin(v) => v,
        };
        let state: u8 = state.into();

        self.connection
            .write_all(&[REPORT_ANALOG | (pin_out + 1), state])?;
        Ok(())
    }

    pub fn analog_write(&mut self, pin: PinId, output: u16) -> Result<()> {
        let pin_out = match pin {
            PinId::Analog(_) => self.pin_id_to_pin(pin),
            PinId::Digital(v) | PinId::Pin(v) => v,
        };
        self.pin_state.pins[pin_out as usize].value = output;
        let bytes_out = output.to_le_bytes();

        self.connection
            .write_all(&[ANALOG_MESSAGE | pin_out, bytes_out[0], bytes_out[1]])?;
        Ok(())
    }

    pub fn digital_write(&mut self, pin: PinId, output: u16) -> Result<()> {
        let pin_out = match pin {
            PinId::Analog(_) => {
                return Err(FirmataError::WrongType(
                    "found digital pin expected analog pin",
                ))
            }
            PinId::Digital(v) | PinId::Pin(v) => v,
        };
        let port = (f64::from(pin_out) / 8_f64).floor() as usize;
        let mut value = 0_i32;
        let mut i = 0;

        self.pin_state.pins[pin_out as usize].value = output;

        while i < 8 {
            if self.pin_state.pins[8 * port + i].value != 0 {
                value |= 1 << i;
            }
            i += 1;
        }
        let bytes_out = value.to_le_bytes();
        self.connection
            .write_all(&[DIGITAL_MESSAGE | port as u8, bytes_out[0], bytes_out[1]])?;
        Ok(())
    }

    pub fn string_write(&mut self, string: &str) -> Result<()> {
        let mut buf: Vec<u8> = vec![START_SYSEX, STRING_DATA];
        for x in string.as_bytes().iter() {
            let double_byte: u16 = (*x).into();
            buf.write_u16::<LittleEndian>(double_byte)?;
        }
        buf.push(END_SYSEX);
        self.connection.write_all(buf.as_slice())?;
        Ok(())
    }

    pub fn set_pin_mode(&mut self, pin: PinId, mode: PinMode) -> Result<()> {
        let pin_out = match pin {
            PinId::Analog(_) => self.pin_id_to_pin(pin),
            PinId::Digital(v) | PinId::Pin(v) => v,
        };
        self.pin_state.pins[pin_out as usize].mode = mode;
        self.connection
            .write_all(&[PIN_MODE, pin_out, mode.to_u8()])?;
        Ok(())
    }

    pub fn read(&mut self, timeout: std::time::Duration) -> Result<MessageId> {
        let message = parser::read_and_parse(&mut self.connection, timeout)?;
        self.handle_message(message.message)?;
        Ok(message.message_id)
    }

    pub fn poll(&mut self, loop_times: usize) -> Result<()> {
        let mut i = 0;
        while i < loop_times {
            match self.read(std::time::Duration::from_secs(1)) {
                Ok(_) => {}
                Err(e) => match e {
                    FirmataError::Timeout(_) => {}
                    e => return Err(e),
                },
            }
            i += 1;
        }
        Ok(())
    }

    pub fn sampling_interval(&mut self, duration: std::time::Duration) -> Result<()> {
        let dur_in_ms: u16 = duration.as_millis() as u16;
        let bytes = dur_in_ms.to_le_bytes();
        self.connection.write_all(&[
            START_SYSEX,
            SAMPLEING_INTERVAL,
            bytes[0],
            bytes[1],
            END_SYSEX,
        ])?;
        Ok(())
    }
}
