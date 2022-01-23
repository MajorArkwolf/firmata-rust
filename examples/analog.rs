use firmata::*;
fn main() {
    let sp = tokio_serial::new("/dev/ttyACM0", 9600).open().unwrap();

    let mut b = firmata::standard::board::Board::new(Box::new(sp));

    let pin = PinId::Analog(0); // first analog pin on board

    println!("firmware version {}", b.firmware_version());
    println!("firmware name {}", b.firmware_name());
    println!("protocol version {}", b.protocol_version());

    b.set_pin_mode(pin, PinMode::Analog).unwrap();

    b.report_analog(pin, true).unwrap();

    loop {
        b.poll(2).unwrap();
        let physical_pin = b.get_physical_pin(pin);
        println!("analog value: {}", physical_pin.value);
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}
