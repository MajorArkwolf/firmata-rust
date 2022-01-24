// These byte constants are defined as part of the firamta protocol
// See https://github.com/firmata/protocol for more info.

// --- Header message bytes ---
pub const START_SYSEX: u8 = 0xF0;
pub const END_SYSEX: u8 = 0xF7;
// Analog messages use a nibble to encode the pin, this means
// the actual value recieved will range from ANALOG_MESSAGE to ANALOG_MESSAGE_END
pub const ANALOG_MESSAGE: u8 = 0xE0;
pub const ANALOG_MESSAGE_END: u8 = 0xEF;
// Digital messages use a nibble to encode the pin, this means
// the actual value recieved will range from DIGITAL_MESSAGE to DIGITAL_MESSAGE_END
pub const DIGITAL_MESSAGE: u8 = 0x90;
pub const DIGITAL_MESSAGE_END: u8 = 0x9F;
pub const PROTOCOL_VERSION: u8 = 0xF9;

// --- Sysex aka System Messages ---
pub const ANALOG_MAPPING_RESPONSE: u8 = 0x6A;
pub const CAPABILITY_RESPONSE: u8 = 0x6C;
pub const I2C_MODE_READ: u8 = 0x01;
pub const REPORT_FIRMWARE: u8 = 0x79;

// --- Message Requests ---
// These are headers used to communicate with the board.
pub const ENCODER_DATA: u8 = 0x61;
pub const ANALOG_MAPPING_QUERY: u8 = 0x69;
pub const CAPABILITY_QUERY: u8 = 0x6B;
pub const PIN_STATE_QUERY: u8 = 0x6D;
pub const PIN_STATE_RESPONSE: u8 = 0x6E;
pub const EXTENDED_ANALOG: u8 = 0x6F;
pub const SERVO_CONFIG: u8 = 0x70;
pub const STRING_DATA: u8 = 0x71;
pub const STEPPER_DATA: u8 = 0x72;
pub const ONEWIRE_DATA: u8 = 0x73;
pub const SHIFT_DATA: u8 = 0x75;
pub const I2C_REQUEST: u8 = 0x76;
pub const I2C_REPLY: u8 = 0x77;
pub const I2C_CONFIG: u8 = 0x78;
pub const I2C_MODE_WRITE: u8 = 0x00;
pub const SAMPLEING_INTERVAL: u8 = 0x7A;
pub const SCHEDULER_DATA: u8 = 0x7B;
pub const SYSEX_NON_REALTIME: u8 = 0x7E;
pub const SYSEX_REALTIME: u8 = 0x7F;
pub const PIN_MODE: u8 = 0xF4;
pub const DIGITAL_PIN_WRITE: u8 = 0xF5;
pub const REPORT_DIGITAL: u8 = 0xD0;
pub const REPORT_ANALOG: u8 = 0xC0;

/// Firmata protocol adds info into the nibbles of certain bytes so we need to verify the range.
/// This function can be used to compare [`DIGITAL_MESSAGE`] to [`DIGITAL_MESSAGE_END`] and
/// [`ANALOG_MESSAGE`] to [`ANALOG_MESSAGE_END`]
pub fn is_id<T: Iterator<Item = u8>>(i: u8, mut s: T) -> bool {
    s.any(|v: u8| v == i)
}
