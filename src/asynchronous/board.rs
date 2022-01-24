use std::sync::Arc;

use super::network::FirmataCodec;
use crate::message::{MessageIn, System};
use crate::{message, FirmataError, PinMode, PinStates, Result};
use futures::SinkExt;
use message::ReportFirmware;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch;
use tokio::sync::{mpsc, Mutex};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, FramedRead, FramedWrite};

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
    DigitalWrite(u8, bool),
    StringWrite(String),
    PinMode(u8, PinMode),
    SampleingInterval(std::time::Duration),
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pin_state: PinStates,
    firmware_name: String,
    firmware_version: String,
    protocol_version: String,
}

pub struct Board<T: AsyncReadExt, U: AsyncWriteExt> {
    pub conn_read: FramedRead<T, FirmataCodec>,
    pub conn_write: FramedWrite<U, FirmataCodec>,
    board_state: State,
    state_tx: watch::Sender<State>,
    state_rx: watch::Receiver<State>,
    message_tx: mpsc::Sender<MessageOut>,
    message_rx: mpsc::Receiver<MessageOut>,
}

impl<
        T: AsyncReadExt + std::marker::Unpin + std::marker::Send,
        U: AsyncWriteExt + std::marker::Unpin + std::marker::Send,
    > Board<T, U>
{
    pub fn create(conn_read: T, conn_write: U) -> Self {
        let conn_read = FramedRead::new(conn_read, FirmataCodec::default());
        let conn_write = FramedWrite::new(conn_write, FirmataCodec::default());
        let board_state = State::default();
        let (state_tx, state_rx) = watch::channel(board_state.clone());
        let (message_tx, message_rx) = mpsc::channel::<MessageOut>(50);
        Self {
            conn_read,
            conn_write,
            board_state,
            state_tx,
            state_rx,
            message_tx,
            message_rx,
        }
    }

    pub fn get_message_publisehr(&self) -> mpsc::Sender<MessageOut> {
        self.message_tx.clone()
    }

    pub fn get_state_subscriber(&self) -> watch::Receiver<State> {
        self.state_rx.clone()
    }

    fn handle_message(&mut self, message: MessageIn) -> Result<()> {
        match message {
            message::MessageIn::Analog(v) => {
                if !self.board_state.pin_state.pins.is_empty() {
                    let pin: usize = self.board_state.pin_state.pin_id_to_u8(v.pin) as usize;
                    if self.board_state.pin_state.pins[pin].analog {
                        self.board_state.pin_state.pins[pin].value = v.value;
                        return Ok(());
                    }
                }
                Err(FirmataError::UninitializedError(
                    "analog message arrived but the pins were not initialised",
                ))
            }
            message::MessageIn::Digital(v) => {
                if !self.board_state.pin_state.pins.is_empty() {
                    for i in 0..8 {
                        let pin = (8 * v.port) + i;

                        if self.board_state.pin_state.pins.len() > pin as usize
                            && self.board_state.pin_state.pins[pin as usize].mode == PinMode::Input
                        {
                            self.board_state.pin_state.pins[pin as usize].value =
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
                    if !self.board_state.pin_state.pins.is_empty() {
                        for id in v.supported_analog_pins {
                            self.board_state.pin_state.pins[id].analog = true;
                        }
                        return Ok(());
                    }
                    Err(FirmataError::UninitializedError(
                        "pins had not been initialised prior to mapping analog pins",
                    ))
                }
                message::System::CapabilityResponseMessage(v) => {
                    self.board_state.pin_state.pins = v.pins;
                    Ok(())
                }
                message::System::ReportFirmwareMessage(v) => {
                    self.board_state.firmware_name = v.name;
                    self.board_state.firmware_version = v.version;
                    Ok(())
                }
                message::System::I2cReplyMessage(_v) => {
                    //self.board_state.i2c_data.push(v.reply);
                    Ok(())
                }
            },
            message::MessageIn::ProtocolVersion(v) => {
                self.board_state.protocol_version = v;
                Ok(())
            }
        }
    }

    pub async fn poll_recv(&mut self) -> Result<()> {
        loop {
            let frame = self.conn_read.next().await;
            let frame = match frame {
                Some(v) => v?,
                None => continue,
            };
            self.handle_message(frame)?;
            self.state_tx.send(self.board_state.clone())?;
        }
    }

    pub async fn poll_send(&mut self) -> Result<()> {
        loop {
            let message = self.message_rx.recv().await.ok_or({
                FirmataError::NotFoundError("all senders were closed which is unexpected")
            })?;
            self.conn_write.send(message).await?
        }
    }

    /// Populates the state of the board, used for quick look ups
    /// # Errors
    /// Can return several firmata errors depeneding on the state that failed.
    pub async fn generate_board_state(&mut self) -> Result<State> {
        self.conn_write.feed(MessageOut::ReportFirmware).await?;
        self.conn_write.feed(MessageOut::CapabilityQuery).await?;
        self.conn_write.feed(MessageOut::AnalogMappingQuery).await?;
        self.conn_write.flush().await?;
        let mut analog_pins: Option<Vec<usize>> = None;
        let mut firmware: Option<ReportFirmware> = None;
        let mut pins: Option<PinStates> = None;
        loop {
            if analog_pins.is_some() && firmware.is_some() && pins.is_some() {
                break;
            }
            let resp = self.conn_read.next().await;
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
            protocol_version: String::default(),
        })
    }
}
