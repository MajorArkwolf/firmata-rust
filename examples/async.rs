extern crate firmata;
use firmata::asynchronous::board::Board;
use firmata::Result;
use tokio::net::TcpStream;

#[tokio::main]
pub async fn main() -> Result<()> {
    let listener = TcpStream::connect("192.168.128.10:3030").await.unwrap();
    let mut board = Board::create(listener);
    let board_state = board.generate_board_state().await?;
    println!("{:?}", board_state);
    Ok(())
}
