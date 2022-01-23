use firmata::*;

fn main() {
    let sp = tokio_serial::new("/dev/ttyACM0", 9600).open().unwrap();

    let mut b = firmata::standard::board::Board::new(Box::new(sp));

    println!("firmware version {}", b.firmware_version());
    println!("firmware name {}", b.firmware_name());
    println!("protocol version {}", b.protocol_version());

    let led = PinId::Digital(13);
    let button_num: u8 = 2;
    let button = PinId::Digital(button_num);

    b.set_pin_mode(led, PinMode::Output).unwrap();
    b.set_pin_mode(button, PinMode::Input).unwrap();

    b.report_digital(button, true).unwrap();

    loop {
        b.poll(1).unwrap();
        if b.pins()[button_num as usize].value == 0 {
            println!("off");
            b.digital_write(led, 0).unwrap();
        } else {
            println!("on");
            b.digital_write(led, 1).unwrap();
        }

        std::thread::sleep(std::time::Duration::from_millis(3));
    }
}
