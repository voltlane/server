use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::Packet;

pub fn configure_performance_tcp_socket(
    stream: &mut TcpStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    stream.set_nodelay(true)?;
    stream.set_linger(Some(std::time::Duration::from_secs(5)))?;
    Ok(())
}

async fn read_exact_cancel_safe<A: AsyncReadExt + std::marker::Unpin>(
    stream: &mut A,
    buf: &mut [u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut total_read = 0;
    while total_read < buf.len() {
        let bytes_read = stream.read(&mut buf[total_read..]).await?;
        if bytes_read == 0 {
            return Err("Connection closed or Eof".into());
        }
        total_read += bytes_read;
    }
    Ok(())
}

async fn write_all_cancel_safe<A: AsyncWriteExt + std::marker::Unpin>(
    stream: &mut A,
    buf: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut total_written = 0;
    while total_written < buf.len() {
        let bytes_written = stream.write(&buf[total_written..]).await?;
        if bytes_written == 0 {
            return Err("Connection closed or Eof".into());
        }
        total_written += bytes_written;
    }
    Ok(())
}

pub async fn recv_size_prefixed<A: AsyncReadExt + std::marker::Unpin>(
    stream: &mut A,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut size_buf = [0u8; 4];
    read_exact_cancel_safe(stream, &mut size_buf).await?;

    let size = u32::from_be_bytes(size_buf) as usize;
    let mut buf = vec![0u8; size];
    read_exact_cancel_safe(stream, &mut buf).await?;

    Ok(buf)
}

pub async fn send_size_prefixed<A: AsyncWriteExt + std::marker::Unpin>(
    stream: &mut A,
    message: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let size = message.len() as u32;
    let size_bytes = size.to_be_bytes();
    let mut combined_message = Vec::with_capacity(4 + message.len());
    combined_message.extend_from_slice(&size_bytes);
    combined_message.extend_from_slice(message);
    write_all_cancel_safe(stream, &combined_message).await?;
    Ok(())
}

pub async fn recv_packet<A: AsyncReadExt + std::marker::Unpin>(
    stream: &mut A,
) -> Result<Packet, Box<dyn std::error::Error + Send + Sync>> {
    let data = recv_size_prefixed(stream).await?;
    if data.len() < 8 {
        return Err("Packet too small".into());
    }
    let client_id = u64::from_be_bytes(data[0..8].try_into().unwrap());
    let buf = data[8..].into();

    Ok(Packet {
        client_id,
        data: buf,
    })
}

pub async fn send_packet<A: AsyncWriteExt + std::marker::Unpin>(
    stream: &mut A,
    packet: Packet,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let size = packet.data.len() as u32 + 8;
    let size_bytes = size.to_be_bytes();
    let client_id_bytes = packet.client_id.to_be_bytes();

    let mut combined_message = Vec::with_capacity(4 + 8 + packet.data.len());
    combined_message.extend_from_slice(&size_bytes);
    combined_message.extend_from_slice(&client_id_bytes);
    combined_message.extend_from_slice(&packet.data);

    write_all_cancel_safe(stream, &combined_message).await?;

    Ok(())
}
