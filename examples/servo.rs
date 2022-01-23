use firmata::*;
fn main() {
    let sp = tokio_serial::new("/dev/ttyACM0", 9600).open().unwrap();

    let mut b = firmata::standard::board::Board::new(Box::new(sp));

    let pin = PinId::Digital(3);

    println!("firmware version {}", b.firmware_version());
    println!("firmware name {}", b.firmware_name());
    println!("protocol version {}", b.protocol_version());

    b.set_pin_mode(pin, PinMode::Servo).unwrap();

    loop {
        for value in 0..180 {
            b.analog_write(pin, value).unwrap();
            println!("{}", value);
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
}
