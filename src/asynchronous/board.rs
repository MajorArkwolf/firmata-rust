use super::boardio::MessageOut::*;
use super::boardio::{MessageOut, State};
use crate::{Pin, PinId, PinMode, Result};
use tokio::sync::mpsc;
use tokio::sync::watch;

pub struct Board {
    state: watch::Receiver<State>,
    tx: mpsc::Sender<MessageOut>,
}

impl Board {
    pub fn create(state: watch::Receiver<State>, tx: mpsc::Sender<MessageOut>) -> Self {
        Self { state, tx }
    }

    fn get_state(&self) -> State {
        self.state.borrow().clone()
    }

    fn convert_pin_id_to_u8(&self, pin: PinId) -> u8 {
        let analog_offset = self.get_state().pin_state.analog_pin_start;
        match pin {
            PinId::Analog(v) => v + analog_offset,
            PinId::Digital(v) => v,
            PinId::Pin(v) => v,
        }
    }

    pub fn pins(&self) -> Vec<Pin> {
        self.get_state().pin_state.pins
    }

    pub fn protocol_version(&self) -> String {
        self.get_state().protocol_version
    }

    pub fn firmware_name(&self) -> String {
        self.get_state().firmware_name
    }

    pub fn firmware_version(&self) -> String {
        self.get_state().firmware_version
    }

    pub async fn query_analog_mapping(&mut self) -> Result<()> {
        self.tx.send(AnalogMappingQuery).await?;
        Ok(())
    }

    pub async fn query_capabilities(&mut self) -> Result<()> {
        self.tx.send(CapabilityQuery).await?;
        Ok(())
    }

    pub async fn query_firmware(&mut self) -> Result<()> {
        self.tx.send(ReportFirmware).await?;
        Ok(())
    }

    //pub async fn i2c_data(&mut self) -> &mut Vec<I2CReply> {
    //    &mut self.i2c_data
    //}

    //pub async fn i2c_config(&mut self, delay: u16) -> Result<()> {
    //    self.tx.send(I2cConfig(delay)).await?;
    //    Ok(())
    //}

    //pub async fn i2c_read(&mut self, address: u8, size: u16) -> Result<()> {
    //    self.tx.send(I2cRead(address, size)).await?;
    //    Ok(())
    //}

    //pub async fn i2c_write(&mut self, address: u8, data: &[u8]) -> Result<()> {
    //    let data: Vec<u8> = data.to_vec();
    //    self.tx.send(I2cWrite(address, data)).await?;
    //    Ok(())
    //}

    pub async fn report_digital(&mut self, pin: PinId, state: bool) -> Result<()> {
        let pin_out = self.convert_pin_id_to_u8(pin);
        self.tx.send(ReportDigital(pin_out, state)).await?;
        Ok(())
    }

    pub async fn report_analog(&mut self, pin: PinId, state: bool) -> Result<()> {
        let pin_out = self.convert_pin_id_to_u8(pin);
        self.tx.send(ReportAnalog(pin_out, state)).await?;
        Ok(())
    }

    pub async fn analog_write(&mut self, pin: PinId, output: u16) -> Result<()> {
        let pin_out = self.convert_pin_id_to_u8(pin);
        self.tx.send(AnalogWrite(pin_out, output)).await?;
        Ok(())
    }

    pub async fn digital_write(&mut self, pin: PinId, output: bool) -> Result<()> {
        let pin_out = self.convert_pin_id_to_u8(pin);
        self.tx.send(DigitalWrite(pin_out, output)).await?;
        Ok(())
    }

    pub async fn string_write(&mut self, string: &str) -> Result<()> {
        self.tx.send(StringWrite(string.to_string())).await?;
        Ok(())
    }

    pub async fn set_pin_mode(&mut self, pin: PinId, mode: PinMode) -> Result<()> {
        let pin_out = self.convert_pin_id_to_u8(pin);
        self.tx.send(PinMode(pin_out, mode)).await?;
        Ok(())
    }

    pub async fn sampling_interval(&mut self, duration: std::time::Duration) -> Result<()> {
        self.tx.send(SampleingInterval(duration)).await?;
        Ok(())
    }
}
