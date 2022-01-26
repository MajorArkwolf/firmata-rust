use super::board::Board;
use super::network::FirmataCodec;
use crate::message::{MessageIn, System};
use crate::{message, FirmataError, PinMode, PinStates, Result};
use futures::SinkExt;
use message::ReportFirmware;
use std::marker::{Send, Unpin};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite};

#[derive(Debug, Clone)]
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
    pub pin_state: PinStates,
    pub firmware_name: String,
    pub firmware_version: String,
    pub protocol_version: String,
}

#[derive(Debug)]
pub struct BoardIo<T: AsyncReadExt, U: AsyncWriteExt> {
    conn_read: FramedRead<T, FirmataCodec>,
    conn_write: FramedWrite<U, FirmataCodec>,
    board_state: State,
    state_tx: watch::Sender<State>,
    state_rx: watch::Receiver<State>,
    message_tx: mpsc::Sender<MessageOut>,
    message_rx: mpsc::Receiver<MessageOut>,
}

impl<T: AsyncReadExt + Unpin + Send, U: AsyncWriteExt + Unpin + Send> BoardIo<T, U> {
    pub fn create(conn_read: T, conn_write: U) -> Self {
        let conn_read = FramedRead::new(conn_read, FirmataCodec::default());
        let conn_write = FramedWrite::new(conn_write, FirmataCodec::default());
        let board_state = State::default();
        let (state_tx, state_rx) = watch::channel(State::default());
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

    pub fn get_board(&self) -> Board {
        Board::create(self.state_rx.clone(), self.message_tx.clone())
    }

    fn update_local(&mut self, message: &MessageOut) {
        match message {
            MessageOut::AnalogWrite(pin, value) => {
                let index: usize = *pin as usize;
                if self.board_state.pin_state.pins.len() > index {
                    self.board_state.pin_state.pins[index].value = *value;
                }
            }
            MessageOut::DigitalWrite(pin, value) => {
                let index: usize = *pin as usize;
                if self.board_state.pin_state.pins.len() > index {
                    self.board_state.pin_state.pins[index].value = *value as u16;
                }
            }
            MessageOut::PinMode(pin, mode) => {
                let index: usize = *pin as usize;
                if self.board_state.pin_state.pins.len() > index {
                    self.board_state.pin_state.pins[index].mode = *mode;
                }
            }
            _ => {}
        }
    }

    fn handle_message(&mut self, message: MessageIn) -> Result<State> {
        match message {
            message::MessageIn::Analog(v) => {
                if !self.board_state.pin_state.pins.is_empty() {
                    let pin: usize = self.board_state.pin_state.pin_id_to_u8(v.pin) as usize;
                    if self.board_state.pin_state.pins[pin].analog {
                        self.board_state.pin_state.pins[pin].value = v.value;
                        Ok(())
                    } else {
                        Err(FirmataError::UninitializedError(
                            "analog message arrived but the pins were not initialised",
                        ))
                    }
                } else {
                    Err(FirmataError::UninitializedError(
                        "analog message arrived but the pins were not initialised",
                    ))
                }
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
                    Ok(())
                } else {
                    Err(FirmataError::UninitializedError(
                        "digital message arrived but the pins were not initialised",
                    ))
                }
            }
            message::MessageIn::System(v) => match v {
                message::System::AnalogMappingResponse(v) => {
                    if !self.board_state.pin_state.pins.is_empty() {
                        for id in v.supported_analog_pins {
                            self.board_state.pin_state.pins[id].analog = true;
                        }
                        Ok(())
                    } else {
                        Err(FirmataError::UninitializedError(
                            "pins had not been initialised prior to mapping analog pins",
                        ))
                    }
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
                    //mutex.i2c_data.push(v.reply);
                    Ok(())
                }
            },
            message::MessageIn::ProtocolVersion(v) => {
                self.board_state.protocol_version = v;
                Ok(())
            }
        }?;
        let new_state = self.board_state.clone();
        Ok(new_state)
    }

    pub async fn poll(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                    val = self.conn_read.next() => {
                        if let Some(v) = val {
                            let new_state = self.handle_message(v?)?;
                            self.state_tx.send(new_state)?;
                        }
                    }
                    val = self.message_rx.recv() => {
                        if let Some(v) = val {
                            self.update_local(&v);
                            self.conn_write.send(v).await?;
                        }
                }
            }
        }
    }

    /// Populates the state of the board, used for quick look ups
    /// # Errors
    /// Can return several firmata errors depeneding on the state that failed.
    pub async fn generate_board_state(&mut self) -> Result<()> {
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

        let new_state = State {
            pin_state,
            firmware_name: firmware.name,
            firmware_version: firmware.version,
            protocol_version: String::default(),
        };

        self.board_state = new_state;
        Ok(())
    }
}
