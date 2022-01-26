//#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(dead_code)]
//! This module contains a client implementation of the
//! [Firmata Protocol](https://github.com/firmata/protocol)
pub mod asynchronous;
pub mod message;
mod protocol_constants;
pub mod standard;
use asynchronous::boardio::{MessageOut, State};
use serde::{Deserialize, Serialize};
use std::iter::Iterator;
use std::marker::Copy;
use std::str;

pub type AnalogPin = u8;
pub type DigitalPin = u8;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum PinId {
    Analog(AnalogPin),
    Digital(DigitalPin),
    Pin(u8),
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum PinMode {
    Input = 0,
    Output = 1,
    Analog = 2,
    Pwm = 3,
    Servo = 4,
    I2c = 6,
    Onewire = 7,
    Stepper = 8,
    Encoder = 9,
    Serial = 10,
    Pullup = 11,
}

impl PinMode {
    const fn to_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Input),
            1 => Ok(Self::Output),
            2 => Ok(Self::Analog),
            3 => Ok(Self::Pwm),
            4 => Ok(Self::Servo),
            6 => Ok(Self::I2c),
            7 => Ok(Self::Onewire),
            8 => Ok(Self::Stepper),
            9 => Ok(Self::Encoder),
            10 => Ok(Self::Serial),
            11 => Ok(Self::Pullup),
            _ => Err(FirmataError::ParseError(
                "failed to convert u8 to pinmode",
                (&[value]).to_vec(),
            )),
        }
    }
}

/// Firmata result type
pub type Result<T> = std::result::Result<T, FirmataError>;
/// Firmata error that wraps all underlying errors for consistency
#[derive(Debug, thiserror::Error)]
pub enum FirmataError {
    #[error("underlying io interrupt {0}")]
    IoError(#[from] std::io::Error),
    #[error("timeout exceeded `{0}` ms")]
    Timeout(String),
    #[error("parse error `{0}`: {:?}")]
    ParseError(&'static str, Vec<u8>),
    #[error("utf8 parse error occured, `{0}`")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("`{0}`")]
    UninitializedError(&'static str),
    #[error("`{0}`")]
    NotFoundError(&'static str),
    #[error("`{0}`")]
    ConversionFailure(&'static str),
    #[error("`{0}`")]
    WrongType(&'static str),
    #[error("State error `{0}`")]
    StateError(&'static str),
    #[error("Out of range error `{0}`")]
    OutOfRange(&'static str),
    #[error("Async State Send Error: `{0}`")]
    AsyncStateSendError(#[from] tokio::sync::watch::error::SendError<State>),
    #[error("Async MessageOut Send Error: `{0}`")]
    AsyncMessageOutSendError(#[from] tokio::sync::mpsc::error::SendError<MessageOut>),
}

/// A structure representing an I2C reply.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct I2CReply {
    pub address: i32,
    pub register: i32,
    pub data: Vec<u8>,
}

/// A structure representing an available pin mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Mode {
    pub mode: PinMode,
    pub resolution: u8,
}

/// A structure representing the current state and configuration of a pin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub modes: Vec<Mode>,
    pub analog: bool,
    pub value: u16,
    pub mode: PinMode,
}

impl Pin {
    /// Converts a byte stream to a valid [`Pin`].
    /// # Errors
    /// The bytestream should contain even amount of bytes since
    /// each part is made of two bytes. Returns [`FirmataError::ConversionFailure`]
    /// if an odd amount of bytes is recieved.
    pub fn deserialize(byte_stream: &[u8]) -> Result<Self> {
        let mut modes: Vec<Mode> = vec![];
        if byte_stream.len() % 2 != 0 {
            return Err(FirmataError::ConversionFailure(
                "odd amount of bytes found when parsing pin, `{0}`",
            ));
        }
        for (i, j) in byte_stream.iter().enumerate() {
            if i % 2 == 0 {
                modes.push(Mode {
                    mode: PinMode::from_u8(*j)?,
                    resolution: byte_stream[i + 1],
                });
            }
        }

        Ok(Self {
            modes,
            analog: false,
            value: 0,
            mode: PinMode::Input,
        })
    }
}

/// A structure representing all available pins on a given board.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PinStates {
    pub pins: Vec<Pin>,
    pub analog_pin_start: u8,
}

impl PinStates {
    #[must_use]
    pub fn create(pins: Vec<Pin>) -> Self {
        Self {
            pins,
            analog_pin_start: 0,
        }
    }

    /// Takes an array of usizes that points to a pin that is analog.
    /// # Errors
    /// Returns [`FirmataError::StateError`] if there is more pins passed
    /// in as a parameter vs pins defined.
    pub fn map_analog_pins(&mut self, analog_pins: Vec<usize>) -> Result<()> {
        if self.pins.len() < analog_pins.len() {
            return Err(FirmataError::StateError(
                "more analog pins then pins inside of pinstate",
            ));
        }
        for id in analog_pins {
            self.pins[id].analog = true;
        }
        Ok(())
    }

    pub fn pin_id_to_u8(&self, pin_id: PinId) -> u8 {
        match pin_id {
            PinId::Analog(v) => v + self.analog_pin_start,
            PinId::Digital(v) | PinId::Pin(v) => v,
        }
    }
}
