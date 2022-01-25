extern crate firmata;
use std::process::Output;

use firmata::asynchronous::board::{Board, MessageOut};
use firmata::{PinId, PinMode, Result};
use futures::{SinkExt, TryFutureExt};
use tokio::net::TcpStream;

#[tokio::main]
pub async fn main() -> Result<()> {
    let (r, w) = TcpStream::connect("192.168.128.10:3030")
        .await
        .unwrap()
        .into_split();
    let mut board = Board::create(r, w);
    let _board_state = board.generate_board_state().await?;

    let publisher = board.get_message_publisher();

    let _x = tokio::task::spawn(async move { board.poll().await });

    let pin = 5;
    publisher
        .send(MessageOut::PinMode(pin, PinMode::Output))
        .await?;

    let mut is_on = true;
    loop {
        println!("{}", is_on);
        let x = publisher.send(MessageOut::DigitalWrite(pin, is_on)).await;
        match x {
            Ok(_) => {}
            Err(_) => break,
        }
        is_on = !is_on;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    Ok(())
}
