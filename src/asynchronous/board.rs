use super::network::FirmataCodec;
use crate::message::{MessageIn, System};
use crate::{message, FirmataError, PinMode, PinStates, Result};
use futures::SinkExt;

use message::ReportFirmware;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

pub enum MessageOut {
    AnalogMappingQuery,
    CapabilityQuery,
    ReportFirmware,
    I2cConfig(u16),
    I2cRead(u8, u16),
    I2cWrite(u8, Vec<u8>),
    ReportDigital(u8, bool),
    ReportAnalog(u8, bool),
    AnalogWrite(u8, u16),
    DigitalMessage(u8, u16),
    StringWrite(String),
    PinMode(u8, PinMode),
    SampleingInterval(std::time::Duration),
}

#[derive(Debug, Clone)]
pub struct State {
    pin_state: PinStates,
    firmware_name: String,
    firmware_version: String,
}

pub struct Board<T: AsyncReadExt + AsyncWriteExt> {
    pub connection: Framed<T, FirmataCodec>,
}

impl<T: AsyncReadExt + AsyncWriteExt + std::marker::Unpin + std::marker::Send> Board<T> {
    pub fn create(conn: T) -> Self {
        let connection = Framed::new(conn, FirmataCodec::new());
        Self { connection }
    }

    /// Populates the state of the board, used for quick look ups
    /// # Errors
    /// Can return several firmata errors depeneding on the state that failed.
    pub async fn generate_board_state(&mut self) -> Result<State> {
        self.connection.feed(MessageOut::ReportFirmware).await?;
        self.connection.feed(MessageOut::CapabilityQuery).await?;
        self.connection.feed(MessageOut::AnalogMappingQuery).await?;
        self.connection.flush().await?;
        let mut analog_pins: Option<Vec<usize>> = None;
        let mut firmware: Option<ReportFirmware> = None;
        let mut pins: Option<PinStates> = None;
        loop {
            if analog_pins.is_some() && firmware.is_some() && pins.is_some() {
                break;
            }
            let resp = self.connection.next().await;
            match resp {
                Some(v) => match v {
                    Ok(msg) => match msg {
                        MessageIn::System(sys_msg) => match sys_msg {
                            System::AnalogMappingResponse(analog_msg) => {
                                analog_pins = Some(analog_msg.supported_analog_pins);
                            }
                            System::CapabilityResponseMessage(cap_msg) => {
                                pins = Some(PinStates::create(cap_msg.pins));
                            }
                            System::ReportFirmwareMessage(firm_msg) => {
                                firmware = Some(firm_msg);
                            }
                            _ => continue,
                        },
                        MessageIn::ProtocolVersion(_prot_msg) => continue,
                        _ => continue,
                    },
                    Err(e) => return Err(e),
                },
                None => continue,
            }
        }

        let mut pin_state = pins.ok_or(FirmataError::WrongType("expected pinstates found none"))?;
        pin_state.map_analog_pins(
            analog_pins.ok_or(FirmataError::WrongType("expected analog pins found none"))?,
        )?;
        let firmware = firmware.ok_or(FirmataError::WrongType("expected firmware found none"))?;

        Ok(State {
            pin_state,
            firmware_name: firmware.name,
            firmware_version: firmware.version,
        })
    }
}
