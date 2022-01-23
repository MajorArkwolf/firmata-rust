use firmata::*;

fn main() {
    let sp = tokio_serial::new("/dev/ttyACM0", 9600).open().unwrap();

    let mut b = firmata::standard::board::Board::new(Box::new(sp));

    println!("firmware version {}", b.firmware_version());
    println!("firmware name {}", b.firmware_name());
    println!("protocol version {}", b.protocol_version());

    let pin = PinId::Digital(13);

    b.set_pin_mode(pin, PinMode::Output).unwrap();

    let mut i = 0;

    loop {
        std::thread::sleep(std::time::Duration::from_millis(200));
        println!("{}", i);
        b.digital_write(pin, i).unwrap();
        i ^= 1;
    }
}
