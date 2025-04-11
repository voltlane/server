use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};

mod args;
mod ids;
mod net;
mod nto1;

async fn simple_master_echo() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut master = TcpStream::connect(("127.0.0.1", 42001)).await?;
    net::configure_performance_tcp_socket(&mut master)?;

    let acceptor = TcpListener::bind(("0.0.0.0", 42000)).await?;
    println!("Listening on {}", acceptor.local_addr()?);

    let (mut client, _) = acceptor.accept().await?;
    net::configure_performance_tcp_socket(&mut client)?;
    loop {
        let msg = net::recv_size_prefixed(&mut client).await?;
        net::send_size_prefixed(&mut master, &msg).await?;
        let msg = net::recv_size_prefixed(&mut master).await?;
        net::send_size_prefixed(&mut client, &msg).await?;
    }

    Ok(())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let args = args::Args::from_env();
    if let Err(e) = args {
        eprintln!("Error: {}. Try --help.", e);
        std::process::exit(1);
    }

    if let Err(err) = simple_master_echo().await {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
