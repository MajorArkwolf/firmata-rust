extern crate firmata;
use std::process::Output;

use firmata::asynchronous::board::{Board, MessageOut};
use firmata::{PinId, PinMode, Result};
use futures::{SinkExt, TryFutureExt};
use tokio::net::TcpStream;

#[tokio::main]
pub async fn main() -> Result<()> {
    let listener = TcpStream::connect("192.168.128.10:3030").await.unwrap();
    let mut board = Board::create(listener);
    let board_state = board.generate_board_state().await?;
    let pin = 5;
    board
        .connection
        .send(MessageOut::PinMode(pin, PinMode::Output))
        .await?;

    let mut isOn = true;

    loop {
        println!("{}", isOn);
        board
            .connection
            .send(MessageOut::DigitalWrite(pin, isOn))
            .await?;
        isOn = !isOn;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
    Ok(())
}
