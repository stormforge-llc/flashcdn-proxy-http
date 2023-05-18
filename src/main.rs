use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use std::io;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    loop {
        let listener = TcpListener::bind("127.0.0.1:3000").await;
        loop {
            let (mut dest, _) = listener.as_ref().unwrap().accept().await?;
            println!("Accepted local connection");
            let mut source = TcpStream::connect("www.cnn.com:443").await?;
            copy_bidirectional(&mut dest, &mut source).await.ok();
        }
    }
}
