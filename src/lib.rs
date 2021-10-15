//! Protocol for transmitting: https://dev.blues.io/notecard/notecard-guides/serial-over-i2c-protocol/
//! API: https://dev.blues.io/reference/notecard-api/introduction/
//!

#![cfg_attr(not(test), no_std)]
#![feature(asm)]

use core::marker::PhantomData;

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use serde::{de::DeserializeOwned, Deserialize};

pub mod card;

#[derive(Debug, defmt::Format)]
pub enum NoteState {
    Handshake,

    /// Ready to make request.
    Request,

    /// Waiting for response to become ready, value is tries made.
    Poll(usize),

    /// Reading response, value is remaining bytes.
    Response(usize),

    /// Full response has been read into `buf`.
    ResponseReady,
}

#[derive(Debug, defmt::Format, Clone)]
pub enum NoteError {
    I2cWriteError,

    I2cReadError,

    DeserError,

    RemainingData,

    /// Method called when notecarrier is in invalid state.
    WrongState,

    NotecardErr(heapless::String<20>),
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
        if !matches!(self.state, NoteState::Response(_)) {
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
                self.buf.clear();
                self.state = NoteState::Response(available);
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
        } else {
            error!("note: data_query called while reading response.");
            Err(NoteError::RemainingData)
        }
    }

    /// Read untill empty.
    fn read(&mut self) -> Result<usize, NoteError> {
        if let NoteState::Response(avail) = self.state {
            // Chunk to read + notecard header (2 bytes)
            let mut bytes = heapless::Vec::<u8, 128>::new();

            let sz = (bytes.capacity() - 2).min(avail);
            bytes.resize(sz + 2, 0).unwrap();

            debug!("asking to read: {} of available {} bytes", sz, avail);

            // Ask for reading `sz` bytes
            self.i2c
                .write(self.addr, &[0, sz as u8])
                .map_err(|_| NoteError::I2cWriteError)?;

            // Read bytes
            self.i2c
                .read(self.addr, &mut bytes)
                .map_err(|_| NoteError::I2cReadError)?;

            let available = bytes[0] as usize;
            let sent = bytes[1] as usize;

            self.buf.extend_from_slice(&bytes[2..]).unwrap(); // XXX: check enough space

            debug!("read: {:?} => {}", &bytes, unsafe {
                core::str::from_utf8_unchecked(&bytes)
            });

            debug!("avail = {}, sent = {}", available, sent);

            if available > 0 {
                self.state = NoteState::Response(available);
            } else {
                self.state = NoteState::ResponseReady;
            }

            Ok(available)
        } else {
            Err(NoteError::WrongState)
        }
    }

    fn take_response(&mut self) -> Result<&[u8], NoteError> {
        if matches!(self.state, NoteState::ResponseReady) {
            self.state = NoteState::Request;

            Ok(&self.buf)
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

#[derive(Deserialize, defmt::Format)]
pub struct NotecardError {
    err: heapless::String<20>,
}

impl From<NotecardError> for NoteError {
    fn from(n: NotecardError) -> NoteError {
        NoteError::NotecardErr(n.err)
    }
}

/// A future response.
///
/// It will not be possible to make any new requests before this has been consumed. If you drop
/// this future before consuming the response the Notecard and driver will be left in inconsistent
/// state. It is not safe to make new requests to the Notecard before the previous response has
/// been read.
#[must_use]
pub struct FutureResponse<
    'a,
    T: DeserializeOwned,
    IOM: Write<SevenBitAddress> + Read<SevenBitAddress>,
> {
    note: &'a mut Note<IOM>,
    _r: PhantomData<T>,
}

impl<'a, T: DeserializeOwned, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>>
    FutureResponse<'a, T, IOM>
{
    fn from(note: &'a mut Note<IOM>) -> FutureResponse<'a, T, IOM> {
        FutureResponse {
            note,
            _r: PhantomData,
        }
    }

    /// Sleep for 25 ms waiting for more data to arrive.
    fn sleep(&self) {
        for _ in 0..1_000_000 {
            unsafe { asm!("nop") }
        }
    }

    /// Reads remaining data and returns the deserialized object if it is ready.
    pub fn poll(&mut self) -> Result<Option<T>, NoteError> {
        match self.note.state {
            NoteState::Poll(_) => {
                // 1. Check for available data
                let sz = self.note.data_query()?;
                if sz > 0 {
                    debug!("response ready: {} bytes..", sz);

                    self.poll()
                } else {
                    // sleep and wait for ready.
                    Ok(None)
                }
            }
            NoteState::Response(_) => {
                let avail = self.note.read()?;
                if avail == 0 {
                    self.poll()
                } else {
                    // sleep and wait for more data.
                    Ok(None)
                }
            }
            NoteState::ResponseReady => {
                debug!("response read, deserializing.");
                let body = self.note.take_response()?;

                if body.starts_with(br##"{"err":"##) {
                    trace!("response is error response, parsing error..");
                    Err(serde_json_core::from_slice::<NotecardError>(body)
                        .map_err(|_| NoteError::DeserError)?
                        .0
                        .into())
                } else {
                    trace!("response is regular, parsing..");
                    Ok(Some(
                        serde_json_core::from_slice::<T>(body)
                            .map_err(|_| NoteError::DeserError)?
                            .0,
                    ))
                }
            }
            _ => Err(NoteError::WrongState),
        }
    }

    /// Wait for response and return raw bytes. These may change on next response,
    /// so this method is probably not staying as it is.
    pub fn wait_raw(mut self) -> Result<&'a [u8], NoteError> {
        loop {
            match self.poll()? {
                Some(_) => return Ok(self.note.take_response()?),
                None => (),
            }

            self.sleep()
        }
    }

    /// Wait for response and return deserialized object.
    pub fn wait(mut self) -> Result<T, NoteError> {
        loop {
            match self.poll()? {
                Some(r) => return Ok(r),
                None => (),
            }

            self.sleep()
        }
    }
}

#[cfg(test)]
mod tests {
}
