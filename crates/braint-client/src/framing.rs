use tokio::io::{AsyncReadExt, AsyncWriteExt};

// NOTE: Manual framing is 20 lines and zero extra deps.
// TODO(phase-2): Evaluate tokio-util::codec::LengthDelimitedCodec when subscriptions arrive.

pub async fn write_frame<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    payload: &[u8],
) -> std::io::Result<()> {
    let len = payload.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(payload).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_frame<R: AsyncReadExt + Unpin>(reader: &mut R) -> std::io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn roundtrip_frame() {
        let mut buf: Vec<u8> = Vec::new();
        write_frame(&mut buf, b"hello").await.unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let read = read_frame(&mut cursor).await.unwrap();
        assert_eq!(read, b"hello");
    }
}
