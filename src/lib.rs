//! Protocol for transmitting: https://dev.blues.io/notecard/notecard-guides/serial-over-i2c-protocol/
//! API: https://dev.blues.io/reference/notecard-api/introduction/
//!

#![no_std]

use core::marker::PhantomData;

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use serde::Deserialize;

pub mod card;

#[derive(Debug, defmt::Format)]
pub enum NoteState {
    Handshake,

    /// Ready to make request.
    Request,

    /// Waiting for response to become ready, value is tries made.
    Poll(usize),

    /// Reading response, value is remaining bytes.
    Response,
}

#[derive(Debug, defmt::Format, Clone, Copy)]
pub enum NoteError {
    I2cWriteError,
    I2cReadError,

    DeserError,

    RemainingData,

    /// Method called when notecarrier is in invalid state.
    WrongState,
}

/// The driver for the Notecard. Remember to intialize before making any requests.
pub struct Note<IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> {
    i2c: IOM,
    addr: u8,
    state: NoteState,
    buf: heapless::Vec<u8, 1024>,
}

impl<IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> Note<IOM> {
    pub fn new(i2c: IOM) -> Note<IOM> {
        Note {
            i2c,
            addr: 0x17,
            state: NoteState::Handshake,
            buf: heapless::Vec::new(),
        }
    }

    /// Initialize the notecard driver by performing handshake with notecard.
    pub fn initialize(&mut self) -> Result<(), NoteError> {
        if matches!(self.state, NoteState::Handshake) {
            self.handshake()
        } else {
            Err(NoteError::WrongState)
        }
    }

    /// Check if notecarrier is connected and responding.
    ///
    /// > This is allowed no matter the state.
    pub fn ping(&mut self) -> bool {
        self.i2c.write(self.addr, &[]).is_ok()
    }

    /// Query the notecard for available bytes.
    ///
    /// > This is allowed no matter the state.
    pub fn data_query(&mut self) -> Result<usize, NoteError> {
        // Ask for reading, but with zero bytes allocated.
        self.i2c
            .write(self.addr, &[0, 0])
            .map_err(|_| NoteError::I2cWriteError)?;

        let mut buf = [0u8; 2];

        // Read available bytes to read
        self.i2c
            .read(self.addr, &mut buf)
            .map_err(|_| NoteError::I2cReadError)?;

        let available = buf[0] as usize;
        let sent = buf[1] as usize;

        if available > 0 {
            self.state = NoteState::Response;
        }

        debug!("avail = {}, sent = {}", available, sent);

        if sent > 0 {
            error!(
                "data query: bytes sent when querying available bytes: {}",
                sent
            );
            Err(NoteError::RemainingData)
        } else {
            Ok(available)
        }
    }

    /// Try to read `buf.len()` bytes from Notecard. Returns bytes read in this chunk. There might
    /// be remaining.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, NoteError> {
        if matches!(self.state, NoteState::Response) {
            debug!("asking to read: {} bytes", buf.len());
            // Ask for reading buf.len() bytes
            self.i2c
                .write(self.addr, &[0, buf.len() as u8])
                .map_err(|_| NoteError::I2cWriteError)?;

            // We need a new buffer because the first two bytes are `available` and `sent`.
            let mut bytes = heapless::Vec::<u8, 280>::new();
            bytes.resize(buf.len() + 2, 0).unwrap();

            // Read bytes
            self.i2c
                .read(self.addr, &mut bytes)
                .map_err(|_| NoteError::I2cReadError)?;

            let available = bytes[0] as usize;
            let sent = bytes[1] as usize;

            debug!("read: {:?} => {}", &bytes, unsafe {
                core::str::from_utf8_unchecked(&bytes)
            });

            debug!("avail = {}, sent = {}", available, sent);

            // if sent > buf.len() {
            //     // more than we asked for.
            //     return Err(NoteError::I2cReadError);
            // }

            buf.copy_from_slice(&bytes[2..]);

            Ok(available)
        } else {
            Err(NoteError::WrongState)
        }
    }

    /// Read any remaining data from the Notecarrier.
    fn consume_response(&mut self) -> Result<(), NoteError> {
        while self.data_query()? > 0 {
            // Consume any left-over response.
        }
        Ok(())
    }

    fn handshake(&mut self) -> Result<(), NoteError> {
        if matches!(self.state, NoteState::Handshake) {
            self.consume_response()?;

            self.state = NoteState::Request;
        }
        Ok(())
    }

    /// Make a request. This returns a `[FutureResponse]` which must be used before making any new
    /// requests. This method is usually called through the API methods like `[card]`.
    pub(crate) fn request(&mut self, cmd: &[u8]) -> Result<(), NoteError> {
        if matches!(self.state, NoteState::Request) {
            debug!("note: making request: {:}", unsafe {
                core::str::from_utf8_unchecked(cmd)
            });

            let mut buf = heapless::Vec::<u8, 255>::new();

            // Send command in chunks of maximum 255 bytes
            for c in cmd.chunks(254) {
                buf.push(c.len() as u8).unwrap();
                buf.extend_from_slice(c).unwrap();

                trace!("note: sending chunk: {:} => {:}", &buf, unsafe {
                    core::str::from_utf8_unchecked(&buf)
                });

                self.i2c
                    .write(self.addr, &buf)
                    .map_err(|_| NoteError::I2cWriteError)?;

                buf.clear();
            }

            // // Terminate command (XXX: include this in last transmission above, if space)
            // buf[0] = 1;
            // buf[1] = b'\n';

            // self.i2c
            //     .write(self.addr, &buf)
            //     .map_err(|_| NoteError::I2cWriteError)?;

            self.state = NoteState::Poll(0);

            Ok(())
        } else {
            Err(NoteError::WrongState)
        }
    }

    /// [card Requests](https://dev.blues.io/reference/notecard-api/card-requests/#card-location)
    pub fn card(&mut self) -> card::Card<IOM> {
        card::Card::from(self)
    }
}

/// A future response.
///
/// It will not be possible to make any new requests before this has been consumed. If you drop
/// this future before consuming the response the Notecard and driver will be left in inconsistent
/// state. It is not safe to make new requests to the Notecard before the previous response has
/// been read.
#[must_use]
pub struct FutureResponse<'a, T, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> {
    note: &'a mut Note<IOM>,
    buf: heapless::Vec<u8, 1024>,
    _r: PhantomData<T>,
}

impl<'b, 'a, T: Deserialize<'b>, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>>
    FutureResponse<'a, T, IOM>
{
    fn from(note: &'a mut Note<IOM>) -> FutureResponse<'a, T, IOM> {
        FutureResponse {
            note,
            buf: heapless::Vec::new(),
            _r: PhantomData,
        }
    }

    /// Sleep for 25 ms waiting for more data to arrive.
    fn sleep(&self) {}

    /// Reads remaining data and returns the deserialized object if it is ready.
    pub fn poll(&'b mut self) -> Result<Option<T>, NoteError> {
        // 1. Check for available data
        let sz = self.note.data_query()?;

        if sz > 0 {
            debug!("response ready, reading {} bytes..", sz);

            if self.buf.len() + sz > self.buf.capacity() {
                // out of space in buffer
                error!("no more space in buffer");
                return Err(NoteError::RemainingData);
            }

            // extend buffer and write from last pos.
            let cur = self.buf.len();
            self.buf.resize(self.buf.len() + sz, 0).unwrap();

            // 2. Read
            let read = self.note.read(&mut self.buf[cur..(cur + sz)])?;

            debug!("read: {} bytes.", read);

            if read < sz {
                warn!("got less than asked for, truncating buf.");
                // We did not get as much as we asked for.
                self.buf.truncate(cur + read);

                Ok(None)
            } else {
                // 3. Deserialize when ready
                debug!("deserializing..");

                let r = serde_json_core::from_slice::<T>(&self.buf)
                    .map_err(|_| NoteError::DeserError)?
                    .0;
                Ok(Some(r))
            }
        } else {
            Ok(None)
        }
    }

    pub fn wait_raw(self) -> &'a [u8] {
        self.note.buf.as_slice()
    }

    // pub fn wait(self) -> Result<T, NoteError> {
    //     // TODO: deserialize
    //     unimplemented!()
    // }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
