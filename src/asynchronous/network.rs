use crate::protocol_constants::{
    is_id, ANALOG_MAPPING_QUERY, ANALOG_MESSAGE, ANALOG_MESSAGE_END, CAPABILITY_QUERY,
    DIGITAL_MESSAGE, DIGITAL_MESSAGE_END, END_SYSEX, I2C_CONFIG, I2C_MODE_READ, I2C_MODE_WRITE,
    I2C_REQUEST, PIN_MODE, PROTOCOL_VERSION, REPORT_ANALOG, REPORT_DIGITAL, REPORT_FIRMWARE,
    SAMPLEING_INTERVAL, START_SYSEX, STRING_DATA,
};

use super::board::MessageOut;
use super::parser::parse_data;
use crate::message::MessageIn;
use crate::{FirmataError, Result};
use byteorder::{LittleEndian, WriteBytesExt};
use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

const BUFFER_SIZE: usize = 1000;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct FirmataCodec(());

impl FirmataCodec {
    #[must_use]
    pub const fn new() -> Self {
        Self(())
    }
}

impl Encoder<MessageOut> for FirmataCodec {
    type Error = FirmataError;

    fn encode(&mut self, item: MessageOut, dst: &mut BytesMut) -> Result<()> {
        match item {
            MessageOut::AnalogMappingQuery => {
                dst.extend_from_slice(&[START_SYSEX, ANALOG_MAPPING_QUERY, END_SYSEX]);
            }
            MessageOut::CapabilityQuery => {
                dst.extend_from_slice(&[START_SYSEX, CAPABILITY_QUERY, END_SYSEX]);
            }
            MessageOut::ReportFirmware => {
                dst.extend_from_slice(&[START_SYSEX, REPORT_FIRMWARE, END_SYSEX]);
            }
            MessageOut::I2cConfig(delay) => {
                let bytes_out = delay.to_le_bytes();
                dst.extend_from_slice(&[
                    START_SYSEX,
                    I2C_CONFIG,
                    bytes_out[0],
                    bytes_out[1],
                    END_SYSEX,
                ]);
            }
            MessageOut::I2cRead(addr, size) => {
                let bytes_out = size.to_le_bytes();
                dst.extend_from_slice(&[
                    START_SYSEX,
                    I2C_REQUEST,
                    addr,
                    (I2C_MODE_READ << 3),
                    bytes_out[0],
                    bytes_out[1],
                    END_SYSEX,
                ]);
            }
            // This method is not fully implemented and requires data to be added after the write.
            MessageOut::I2cWrite(addr, _data) => dst.extend_from_slice(&[
                START_SYSEX,
                I2C_REQUEST,
                addr,
                I2C_MODE_WRITE << 3,
                END_SYSEX,
            ]),
            MessageOut::ReportDigital(pin, enable) => {
                dst.extend_from_slice(&[REPORT_DIGITAL | pin, enable as u8]);
            }
            MessageOut::ReportAnalog(pin, enable) => {
                dst.extend_from_slice(&[REPORT_ANALOG | (pin + 1), enable as u8]);
            }
            MessageOut::AnalogWrite(pin, output) => {
                let bytes_out = output.to_le_bytes();
                dst.extend_from_slice(&[ANALOG_MESSAGE | pin, bytes_out[0], bytes_out[1]]);
            }
            MessageOut::DigitalMessage(port, output) => {
                let bytes_out = output.to_le_bytes();
                dst.extend_from_slice(&[DIGITAL_MESSAGE | port as u8, bytes_out[0], bytes_out[1]]);
            }
            MessageOut::StringWrite(string_out) => {
                let mut buf: Vec<u8> = vec![START_SYSEX, STRING_DATA];
                for x in string_out.as_bytes().iter() {
                    let double_byte: u16 = (*x).into();
                    buf.write_u16::<LittleEndian>(double_byte)?;
                }
                buf.push(END_SYSEX);
                dst.extend_from_slice(buf.as_slice());
            }
            MessageOut::PinMode(pin, mode) => dst.extend_from_slice(&[PIN_MODE, pin, mode as u8]),
            MessageOut::SampleingInterval(duration) => {
                let dur_in_ms: u16 = duration.as_millis() as u16;
                let bytes = dur_in_ms.to_le_bytes();
                dst.extend_from_slice(&[
                    START_SYSEX,
                    SAMPLEING_INTERVAL,
                    bytes[0],
                    bytes[1],
                    END_SYSEX,
                ]);
            }
        }
        Ok(())
    }
}

fn remove_section(buffer: &mut BytesMut, start_index: usize, end_index: usize) -> BytesMut {
    let mut new_buffer = buffer.split_off(start_index);
    buffer.unsplit(new_buffer.split_off(end_index + 1));
    new_buffer
}

impl Decoder for FirmataCodec {
    type Item = MessageIn;
    type Error = FirmataError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        let start = src.iter().position(|x| {
            *x == START_SYSEX
                || is_id(*x, DIGITAL_MESSAGE..DIGITAL_MESSAGE_END)
                || is_id(*x, ANALOG_MESSAGE..ANALOG_MESSAGE_END)
                || *x == PROTOCOL_VERSION
        });

        // Tries to get an entire message from the buffer
        let bytes: Option<BytesMut> = match start {
            Some(v) => {
                if src[v] == START_SYSEX {
                    let end = src.iter().position(|y| *y == END_SYSEX);
                    end.map(|end_val| remove_section(src, v, end_val))
                } else if is_id(src[v], ANALOG_MESSAGE..ANALOG_MESSAGE_END)
                    || is_id(src[v], DIGITAL_MESSAGE..DIGITAL_MESSAGE_END)
                    || src[v] == PROTOCOL_VERSION
                {
                    if src.len() > (v + 2) {
                        Some(remove_section(src, v, v + 2 + 1))
                    } else {
                        // this should grow because we are missing two bytes
                        None
                    }
                } else {
                    None
                }
            }
            None => None,
        };

        if src.len() > (BUFFER_SIZE as f64 * 0.7) as usize {
            if let Some(value) = src.iter().position(|x| {
                *x == START_SYSEX
                    || *x == ANALOG_MESSAGE
                    || *x == DIGITAL_MESSAGE
                    || *x == PROTOCOL_VERSION
            }) {
                if value > (BUFFER_SIZE as f64 * 0.3) as usize {
                    let _x = src.split_to(value);
                }
            }
        }

        match bytes {
            Some(mut data) => Ok(Some(parse_data(&mut data)?)),
            None => Ok(None),
        }
    }

    fn framed<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Sized>(
        self,
        io: T,
    ) -> tokio_util::codec::Framed<T, Self>
    where
        Self: Sized,
    {
        tokio_util::codec::Framed::new(io, self)
    }
}
