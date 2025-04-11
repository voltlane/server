use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub fn configure_performance_tcp_socket(
    stream: &mut TcpStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    stream.set_nodelay(true)?;
    Ok(())
}

pub async fn recv_size_prefixed<A: AsyncReadExt + std::marker::Unpin>(
    stream: &mut A,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut size_buf = [0u8; 4];
    stream.read_exact(&mut size_buf).await?;

    let size = u32::from_be_bytes(size_buf) as usize;
    let mut buf = Vec::with_capacity(size);
    unsafe { buf.set_len(size) };
    stream.read_exact(&mut buf).await?;

    Ok(buf)
}

pub async fn send_size_prefixed<A: AsyncWriteExt + std::marker::Unpin>(
    stream: &mut A,
    message: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let size = message.len() as u32;
    let size_bytes = size.to_be_bytes();

    let bufs = [
        std::io::IoSlice::new(&size_bytes),
        std::io::IoSlice::new(message),
    ];

    stream.write_vectored(&bufs).await?;

    Ok(())
}
