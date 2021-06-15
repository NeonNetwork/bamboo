use crate::{packet::Packet, StreamReader, StreamWriter};

use aes::{
  cipher::{AsyncStreamCipher, NewCipher},
  Aes128,
};
use cfb8::Cfb8;
use common::{util, util::Buffer, version::ProtocolVersion};
use miniz_oxide::{deflate::compress_to_vec_zlib, inflate::decompress_to_vec_zlib};
use ringbuf::{Consumer, Producer, RingBuffer};
use std::{
  io,
  io::{ErrorKind, Result},
  net::TcpStream as StdTcpStream,
};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
  },
};

pub struct JavaStreamReader {
  stream:      OwnedReadHalf,
  prod:        Producer<u8>,
  cons:        Consumer<u8>,
  // If this is zero, compression is disabled.
  compression: usize,
  // If this is none, then encryption is disabled.
  cipher:      Option<Cfb8<Aes128>>,
}
pub struct JavaStreamWriter {
  stream:      OwnedWriteHalf,
  // If this is zero, compression is disabled.
  compression: usize,
  // If this is none, then encryption is disabled.
  cipher:      Option<Cfb8<Aes128>>,
}

pub fn new(stream: StdTcpStream) -> Result<(JavaStreamReader, JavaStreamWriter)> {
  // We want to block on read calls
  // stream.set_nonblocking(true)?;
  let (read, write) = TcpStream::from_std(stream)?.into_split();
  Ok((JavaStreamReader::new(read), JavaStreamWriter::new(write)))
}

impl JavaStreamReader {
  pub fn new(stream: OwnedReadHalf) -> Self {
    let buf = RingBuffer::new(1024);
    let (prod, cons) = buf.split();
    StreamReader { stream, prod, cons, compression: 0, cipher: None }
  }
  pub fn set_compression(&mut self, compression: i32) {
    self.compression = compression as usize;
  }
  pub fn enable_encryption(&mut self, secret: &[u8; 16]) {
    self.cipher = Some(Cfb8::new_from_slices(secret, secret).unwrap());
  }
}

#[async_trait]
impl StreamReader for JavaStreamReader {
  async fn poll(&mut self) -> Result<()> {
    let mut msg = vec![];

    // This appends to msg, so we don't need to truncate
    let n = self.stream.read_buf(&mut msg).await?;
    if n == 0 {
      return Err(io::Error::new(ErrorKind::ConnectionAborted, "client has disconnected"));
    }
    if let Some(c) = &mut self.cipher {
      c.decrypt(&mut msg);
    }
    self.prod.push_slice(&msg);
    Ok(())
  }
  fn read(&mut self, ver: ProtocolVersion) -> Result<Option<Packet>> {
    let mut len = 0;
    let mut read = -1;
    self.cons.access(|a, _| {
      let (a, b) = util::read_varint(a);
      len = a as isize;
      read = b;
    });
    // Varint that is more than 5 bytes long.
    if read < 0 {
      return Err(io::Error::new(ErrorKind::InvalidData, "invalid varint"));
    }
    // Incomplete varint, or an incomplete packet
    if read == 0 || len > self.cons.len() as isize {
      return Ok(None);
    }
    // Now that we know we have a valid packet, we pop the length bytes
    self.cons.discard(read as usize);
    let mut vec = vec![0; len as usize];
    self.cons.pop_slice(&mut vec);
    // And parse it
    if self.compression != 0 {
      let mut buf = Buffer::new(vec);
      let uncompressed_length = buf.read_varint();
      if uncompressed_length == 0 {
        Ok(Some(Packet::from_buf(buf.read_all(), ver)))
      } else {
        let decompressed = decompress_to_vec_zlib(&buf.read_all()).map_err(|e| {
          io::Error::new(ErrorKind::InvalidData, format!("invalid zlib data: {:?}", e))
        })?;
        Ok(Some(Packet::from_buf(decompressed, ver)))
      }
    } else {
      Ok(Some(Packet::from_buf(vec, ver)))
    }
  }
}

impl JavaStreamWriter {
  pub fn new(stream: OwnedWriteHalf) -> Self {
    JavaStreamWriter { stream, compression: 0, cipher: None }
  }
  pub fn set_compression(&mut self, compression: i32) {
    self.compression = compression as usize;
  }
  pub fn enable_encryption(&mut self, secret: &[u8; 16]) {
    self.cipher = Some(Cfb8::new_from_slices(secret, secret).unwrap());
  }

  async fn write_data(&mut self, data: &mut [u8]) -> Result<()> {
    if let Some(c) = &mut self.cipher {
      c.encrypt(data);
    }
    self.stream.write(data).await?;
    Ok(())
  }

  pub async fn write(&mut self, p: Packet) -> Result<()> {
    // This is the packet, including it's id
    let mut bytes = p.serialize();

    // Either the uncompressed length, or the total and uncompressed length.
    let mut buf = Buffer::new(vec![]);

    if self.compression != 0 {
      if bytes.len() > self.compression {
        let uncompressed_length = bytes.len();
        let mut compressed = compress_to_vec_zlib(&bytes, 1);

        // See how many bytes the uncompressed_length varint takes up
        let mut uncompressed_length_buf = Buffer::new(vec![]);
        uncompressed_length_buf.write_varint(uncompressed_length as i32);

        // This is the total length of the packet.
        let total_length = uncompressed_length_buf.len() + compressed.len();
        buf.write_varint(total_length as i32);
        buf.write_varint(uncompressed_length as i32);
        self.write_data(&mut buf).await?;
        self.write_data(&mut compressed).await?;
      } else {
        // The 1 is for the zero uncompressed_length
        buf.write_varint(bytes.len() as i32 + 1);
        buf.write_varint(0);
        self.write_data(&mut buf).await?;
        self.write_data(&mut bytes).await?;
      }
    } else {
      // Uncompressed packets just have the length prefixed.
      buf.write_varint(bytes.len() as i32);
      self.write_data(&mut buf).await?;
      self.write_data(&mut bytes).await?;
    }

    Ok(())
  }
}
