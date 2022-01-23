#Firmata

Control your [firmata](https://github.com/firmata/protocol) powered device with rust this rust application.

Getting Started
---
```bash
$ git clone https://github.com/MajorArkwolf/firmata-rust.git
$ cd rust-firmata
$ cargo build
$ cargo run --example blink
```
Usage
---
Add `firmata` to  your `Cargo.toml`
```
[dependencies]
firmata = "0.0.1"
```

Implemented
---
- Async
- Analog
- Digital
- Servo
- String write
- Sampling Interval
- I2C
- Pwm 
