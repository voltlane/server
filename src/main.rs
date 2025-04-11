use std::sync::Arc;

use tokio::{
    net::{TcpListener, TcpStream},
    select,
};

mod args;
mod ids;
mod net;
mod nto1;

async fn simple_master_echo() -> Result<(), Box<dyn std::error::Error>> {
    use tokio::io::AsyncWriteExt;

    let mut master = TcpStream::connect(("127.0.0.1", 42001)).await?;
    net::configure_performance_tcp_socket(&mut master)?;

    let acceptor = TcpListener::bind(("0.0.0.0", 42000)).await?;
    println!("Listening on {}", acceptor.local_addr()?);

    let mut manager = nto1::Manager::new();

    loop {
        if manager.is_empty() {
            println!("No clients connected, waiting for a connection...");
            if let Ok((mut client, _)) = acceptor.accept().await {
                net::configure_performance_tcp_socket(&mut client)?;
                let id = manager.add_client(client);
                println!("Client {} connected", id);
            }
        } else {
            select! {
                Ok((mut client, _)) = acceptor.accept() => {
                    net::configure_performance_tcp_socket(&mut client)?;
                    let id = manager.add_client(client);
                    println!("Client {} connected", id);
                }
                Ok((id, msg)) = manager.recv_message() => {
                    // println!("Received message from client {}: {:?}", id, msg);
                    let msg = Arc::new(msg);
                    if let Err(e) = net::send_size_prefixed(&mut master, msg.as_slice()).await {
                        eprintln!("Error sending message to master: {}", e);
                        break;
                    }
                }
                Ok(msg) = net::recv_size_prefixed(&mut master) => {
                    if let Err(e) = manager.send_message(msg).await {
                        eprintln!("Error sending message to client(s): {:#?}", e);
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
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
