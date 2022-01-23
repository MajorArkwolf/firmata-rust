#Firmata

Control your [firmata](https://github.com/firmata/protocol) powered device with rust!

Getting Started
---
```bash
$ git clone https://github.com/zankich/rust-firmata && cd rust-firmata
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

What works
---
- I2C
- Pwm 
- Servo
- Analog
- Digital

