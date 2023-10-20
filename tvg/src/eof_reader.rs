use std::io::{self, Read};

const PEEK_BUF_HALF_SIZE: usize = 32;
const PEEK_BUF_SIZE: usize = PEEK_BUF_HALF_SIZE * 2;
pub struct EofReader<R> {
    read: R,
    buf: [u8; PEEK_BUF_SIZE],
    buf_pos: usize,
    buf_read_pos: usize,
    at_eof: bool,
}

impl<R: Read> EofReader<R> {
    pub fn new(read: R) -> io::Result<Self> {
        let mut reader = Self {
            read,
            buf: [0; PEEK_BUF_SIZE],
            buf_pos: 0,
            buf_read_pos: 0,
            at_eof: false,
        };
        reader.fill_if_needed()?;
        Ok(reader)
    }
    fn fill_if_needed(&mut self) -> io::Result<()> {
        if self.at_eof {
            return Ok(());
        }

        if self.buf_pos >= PEEK_BUF_HALF_SIZE {
            let (a, b) = self.buf.split_at_mut(PEEK_BUF_HALF_SIZE);
            a.copy_from_slice(b);
            self.buf_pos -= PEEK_BUF_HALF_SIZE;
            self.buf_read_pos -= PEEK_BUF_HALF_SIZE;
        }

        if self.buf_read_pos <= PEEK_BUF_HALF_SIZE {
            let read_count = self.read.read(&mut self.buf[self.buf_read_pos..])?;
            if read_count == 0 {
                self.at_eof = true;
            } else {
                self.buf_read_pos += read_count;
            }
        }

        Ok(())
    }

    pub fn is_at_eof(&self) -> bool {
        self.buf_pos == self.buf_read_pos && self.at_eof
    }

    pub fn peek_tag(&mut self, buf: &mut [u8; 4]) -> io::Result<usize> {
        // fill twice (once here, once in read) so that it must have at least 2 bytes available
        self.fill_if_needed()?;
        let read = self.read(buf)?;
        self.buf_pos -= read;
        Ok(read)
    }
}

impl<R: Read> Read for EofReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.fill_if_needed()?;
        if self.buf_pos < self.buf_read_pos {
            let ready_len = self.buf_read_pos - self.buf_pos;
            let read_len = buf.len().min(ready_len);
            buf[..read_len].copy_from_slice(&self.buf[self.buf_pos..(self.buf_pos + read_len)]);
            self.buf_pos += read_len;
            Ok(read_len)
        } else if self.at_eof {
            Ok(0)
        } else {
            Err(io::Error::new(io::ErrorKind::UnexpectedEof, "invalid state probably"))
        }
    }
}
