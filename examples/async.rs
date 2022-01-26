extern crate firmata;

use firmata::asynchronous::boardio::BoardIo;
use firmata::{PinId, PinMode, Result};
use tokio::net::TcpStream;

#[tokio::main]
pub async fn main() -> Result<()> {
    let (r, w) = TcpStream::connect("192.168.128.10:3030")
        .await
        .unwrap()
        .into_split();
    let mut board = BoardIo::create(r, w);
    board.generate_board_state().await?;

    let mut board_communicator = board.get_board();
    let mut board_communicator2 = board.get_board();

    // Backend IO
    let _x = tokio::task::spawn(async move { board.poll().await });

    // Job running in parrallel as a task
    let _y = tokio::task::spawn(async move {
        let pin = PinId::Digital(5);
        board_communicator2
            .set_pin_mode(pin, PinMode::Output)
            .await
            .unwrap();
        let mut is_on = true;
        loop {
            println!("{}", is_on);
            board_communicator2.digital_write(pin, is_on).await.unwrap();
            is_on = !is_on;
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    });

    // Main task
    let pin = PinId::Digital(6);
    board_communicator
        .set_pin_mode(pin, PinMode::Output)
        .await?;

    let mut is_on = true;
    loop {
        println!("{}", is_on);
        board_communicator.digital_write(pin, is_on).await?;
        is_on = !is_on;
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }
    Ok(())
}
