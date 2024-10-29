//! Protocol for transmitting: <https://dev.blues.io/notecard/notecard-guides/serial-over-i2c-protocol/>
//! API: <https://dev.blues.io/reference/notecard-api/introduction/>
//!
#![feature(type_changing_struct_update)]
#![cfg_attr(not(test), no_std)]

use core::convert::Infallible;
use core::marker::PhantomData;

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use heapless::{String, Vec};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub mod card;
pub mod dfu;
pub mod hub;
pub mod note;
pub mod web;

/// Delay between polling for new response.
const RESPONSE_DELAY: u16 = 25;

/// The size of the shared request and receive buffer. Requests and responses may not serialize to
/// any greater value than this.
pub const DEFAULT_BUF_SIZE: usize = 18 * 1024;

#[derive(Debug, defmt::Format)]
pub struct NotecardConfig {
    /// I2C address of Notecard.
    pub i2c_addr: u8,

    /// Timeout while waiting for response (ms).
    pub response_timeout: u16,

    /// Delay between chunks when transmitting (ms).
    ///
    /// See note on `segment_delay`.
    ///
    /// > `note-c`: https://github.com/blues/note-c/blob/master/n_lib.h#L52
    /// > Original: 20 ms
    pub chunk_delay: u16,

    /// Delay between segments when transmitting (ms).
    ///
    /// > These delay may be almost eliminated for Notecard firmware version 3.4 (and presumably
    /// above).
    ///
    /// > `note-c`: https://github.com/blues/note-c/blob/master/n_lib.h#L46
    /// > Original: 250 ms.
    pub segment_delay: u16,
}

impl Default for NotecardConfig {
    fn default() -> Self {
        NotecardConfig {
            i2c_addr: 0x17,
            response_timeout: 5000,
            chunk_delay: 20,
            segment_delay: 250,
        }
    }
}

#[derive(Debug, defmt::Format)]
pub enum NoteState {
    /// Perform handshake with Notecard.
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

    DeserError(String<256>),

    SerError,

    /// Request does not end with '\n'.
    InvalidRequest,

    RemainingData,

    TimeOut,

    BufOverflow,

    /// Method called when notecarrier is in invalid state.
    WrongState,

    /// Notecard firmware is being updated.
    DFUInProgress,

    NotecardErr(String<256>),
}

impl NoteError {
    pub fn new_desererror(msg: &[u8]) -> NoteError {
        let msg = core::str::from_utf8(&msg).unwrap_or("[invalid utf-8]");
        let mut s = String::new();
        s.push_str(msg).ok();
        NoteError::DeserError(s)
    }

    pub fn string_err(_e: Infallible) -> NoteError {
        NoteError::BufOverflow
    }
}

pub(crate) fn str_string<const N: usize>(
    a: Option<&str>,
) -> Result<Option<heapless::String<N>>, NoteError> {
    a.map(heapless::String::try_from)
        .transpose()
        .map_err(NoteError::string_err)
}

#[derive(Deserialize, defmt::Format)]
pub struct NotecardError {
    err: String<256>,
}

impl From<NotecardError> for NoteError {
    fn from(n: NotecardError) -> NoteError {
        if n.err.contains("{dfu-in-progress}") {
            NoteError::DFUInProgress
        } else {
            NoteError::NotecardErr(n.err)
        }
    }
}

/// The driver for the Notecard. Must be intialized before making any requests.
pub struct Notecard<
    IOM: Write<SevenBitAddress> + Read<SevenBitAddress>,
    const BUF_SIZE: usize = DEFAULT_BUF_SIZE,
> {
    i2c: IOM,
    addr: u8,
    state: NoteState,

    /// The receive buffer. Must be large enough to hold the largest response that will be received.
    buf: Vec<u8, BUF_SIZE>,

    response_timeout: u16,
    chunk_delay: u16,
    segment_delay: u16,
}

pub struct SuspendState<const BUF_SIZE: usize> {
    addr: u8,
    state: NoteState,
    buf: Vec<u8, BUF_SIZE>,
    response_timeout: u16,
    chunk_delay: u16,
    segment_delay: u16,
}

impl<IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BUF_SIZE: usize>
    Notecard<IOM, BUF_SIZE>
{
    pub fn new(i2c: IOM) -> Notecard<IOM, BUF_SIZE> {
        Self::new_with_config(i2c, NotecardConfig::default())
    }

    pub fn new_with_config(i2c: IOM, c: NotecardConfig) -> Notecard<IOM, BUF_SIZE> {
        Notecard {
            i2c,
            addr: c.i2c_addr,
            state: NoteState::Handshake,
            buf: Vec::new(),

            response_timeout: c.response_timeout,
            chunk_delay: c.chunk_delay,
            segment_delay: c.segment_delay,
        }
    }

    /// Resize the internal buffer, consuming the existing, and returning a new Notecard
    /// instance.
    pub fn resize_buf<const B: usize>(self) -> Result<Notecard<IOM, B>, NoteError> {
        if B < self.buf.len() {
            Err(NoteError::BufOverflow)
        } else {
            Ok(Notecard {
                buf: Vec::<_, B>::from_slice(&self.buf).unwrap(),
                ..self
            })
        }
    }

    /// Free the IOM device and return the driver state so that it can be quickly resumed. It is
    /// not safe to change the state of the Notecard in the meantime, or create a second driver
    /// without using this state.
    pub fn suspend(self) -> (IOM, SuspendState<BUF_SIZE>) {
        (
            self.i2c,
            SuspendState {
                state: self.state,
                buf: self.buf,
                addr: self.addr,
                response_timeout: self.response_timeout,
                chunk_delay: self.chunk_delay,
                segment_delay: self.segment_delay,
            },
        )
    }

    /// Resume a previously [`suspend`]ed Notecard driver.
    pub fn resume(i2c: IOM, state: SuspendState<BUF_SIZE>) -> Notecard<IOM, BUF_SIZE> {
        Notecard {
            i2c,
            addr: state.addr,
            state: state.state,
            buf: state.buf,
            response_timeout: state.response_timeout,
            chunk_delay: state.chunk_delay,
            segment_delay: state.segment_delay,
        }
    }

    /// Initialize the notecard driver by performing handshake with notecard.
    pub fn initialize(&mut self, delay: &mut impl DelayMs<u16>) -> Result<(), NoteError> {
        info!("note: initializing.");
        self.reset(delay)
    }

    /// Check if notecarrier is connected and responding.
    ///
    /// > This is allowed no matter the state.
    pub fn ping(&mut self) -> bool {
        self.i2c.write(self.addr, &[]).is_ok()
    }

    /// Query the notecard for available bytes.
    pub fn data_query(&mut self) -> Result<usize, NoteError> {
        trace!("note: data_query: {:?}", self.state);
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

            trace!("avail = {}, sent = {}", available, sent);

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
            Err(NoteError::WrongState)
        }
    }

    /// Read until empty.
    fn read(&mut self) -> Result<usize, NoteError> {
        if let NoteState::Response(avail) = self.state {
            // Chunk to read + notecard header (2 bytes)
            let mut bytes = Vec::<u8, 128>::new();

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

            trace!("read:  {}", unsafe {
                core::str::from_utf8_unchecked(&bytes)
            });

            trace!("avail = {}, sent = {}", available, sent);

            if available > 0 {
                self.state = NoteState::Response(available);
            } else {
                self.state = NoteState::ResponseReady;
            }

            Ok(available)
        } else {
            error!("read: called when not waiting for response");
            Err(NoteError::WrongState)
        }
    }

    /// Take the response from the buffer. Once this function has been called, the state is reset
    /// and it is no longer safe to read the buffer.
    ///
    /// Safety:
    ///
    /// This function returns an immutable reference to the buffer, but new requests require a
    /// mutable reference to `Note`. This is not granted before the immutable reference is
    /// released.
    fn take_response(&mut self) -> Result<&[u8], NoteError> {
        if matches!(self.state, NoteState::ResponseReady) {
            self.state = NoteState::Request;

            Ok(&self.buf)
        } else {
            error!("take response called when response not ready");
            Err(NoteError::WrongState)
        }
    }

    /// Poll for data.
    fn poll(&mut self) -> Result<Option<&[u8]>, NoteError> {
        trace!("note: poll: {:?}", self.state);
        match self.state {
            NoteState::Poll(_) => {
                // 1. Check for available data
                let sz = self.data_query()?;
                if sz > 0 {
                    debug!("response ready: {} bytes..", sz);

                    self.poll()
                } else {
                    // sleep and wait for ready.
                    Ok(None)
                }
            }
            NoteState::Response(_) => {
                let avail = self.read()?;
                if avail == 0 {
                    self.poll()
                } else {
                    // sleep and wait for more data.
                    Ok(None)
                }
            }
            NoteState::ResponseReady => {
                debug!("response read, deserializing.");
                Ok(Some(self.take_response()?))
            }
            _ => {
                error!("poll called when not receiving response");
                Err(NoteError::WrongState)
            }
        }
    }

    /// Read any remaining data from the Notecarrier. This will cancel any waiting responses, and
    /// waiting for a response after this call will time-out.
    unsafe fn consume_response(&mut self, delay: &mut impl DelayMs<u16>) -> Result<(), NoteError> {
        warn!("note: trying to consume any left-over response.");
        let mut waited = 0;

        while waited < self.response_timeout {
            if matches!(self.poll()?, Some(_)) {
                self.buf.clear();
                return Ok(());
            }

            delay.delay_ms(RESPONSE_DELAY);
            waited += RESPONSE_DELAY;
        }

        self.buf.clear();

        error!("response timed out (>= {}).", self.response_timeout);
        Err(NoteError::TimeOut)
    }

    /// Reset notecard driver and state. Any waiting responses will be invalidated
    /// and time-out. However, you won't be able to get a mutable reference without having
    /// dropped the `FutureResponse`.
    pub fn reset(&mut self, delay: &mut impl DelayMs<u16>) -> Result<(), NoteError> {
        warn!("resetting: consuming any left-over response and perform a new handshake.");

        self.buf.clear(); // clear in case data_query() is 0.
        self.state = NoteState::Handshake;
        self.handshake(delay)
    }

    fn handshake(&mut self, delay: &mut impl DelayMs<u16>) -> Result<(), NoteError> {
        if matches!(self.state, NoteState::Handshake) {
            debug!("note: handshake");
            if self.data_query()? > 0 {
                error!("note: handshake: remaining data in queue, consuming..");
                unsafe { self.consume_response(delay)? };
            }

            self.state = NoteState::Request;
        }
        Ok(())
    }

    /// Sends request from buffer.
    fn send_request(&mut self, delay: &mut impl DelayMs<u16>) -> Result<(), NoteError> {
        // This is presumably limited by the notecard firmware.
        const CHUNK_LENGTH_MAX: usize = 127;
        // This is a limit that was required on some Arduinos. Can probably be increased up to
        // `CHUNK_LENGTH_MAX`. Should maybe be configurable.
        const CHUNK_LENGTH_I: usize = 30;
        const CHUNK_LENGTH: usize = if CHUNK_LENGTH_I < CHUNK_LENGTH_MAX {
            CHUNK_LENGTH_I
        } else {
            CHUNK_LENGTH_MAX
        };

        // `note-c` uses `250` for `SEGMENT_LENGTH`. Round to closest divisible
        // by CHUNK_LENGTH so that we don't end up with unnecessarily fragmented
        // chunks. https://github.com/blues/note-c/blob/master/n_lib.h#L40 .
        const SEGMENT_LENGTH: usize = (250 / CHUNK_LENGTH) * CHUNK_LENGTH;

        if !matches!(self.state, NoteState::Request) {
            warn!("note: request: wrong-state, resetting before new request.");
            self.reset(delay)?;
        }

        if self.buf.last() != Some(&b'\n') {
            return Err(NoteError::InvalidRequest);
        }

        trace!("note: making request: {}", unsafe {
            core::str::from_utf8_unchecked(&self.buf)
        });

        let mut buf = Vec::<u8, { CHUNK_LENGTH + 1 }>::new();
        for segment in self.buf.chunks(SEGMENT_LENGTH) {
            for c in segment.chunks(buf.capacity() - 1) {
                buf.push(c.len() as u8).unwrap();
                buf.extend_from_slice(c).unwrap();

                trace!("note: sending chunk: {} => {}", &buf, unsafe {
                    core::str::from_utf8_unchecked(&buf)
                });

                self.i2c
                    .write(self.addr, &buf)
                    .map_err(|_| NoteError::I2cWriteError)?;

                buf.clear();
                delay.delay_ms(self.chunk_delay);
            }
            delay.delay_ms(self.segment_delay);
        }

        self.state = NoteState::Poll(0);

        Ok(())
    }

    /// Make a raw request. The byte slice must end with `\n`. After making a request a
    /// [FutureResponse] must be created and consumed.
    pub(crate) fn request_raw(
        &mut self,
        delay: &mut impl DelayMs<u16>,
        cmd: &[u8],
    ) -> Result<(), NoteError> {
        self.buf.clear();
        self.buf
            .resize(cmd.len(), 0)
            .map_err(|_| NoteError::BufOverflow)?;
        let buf: &mut [u8] = self.buf.as_mut();
        buf.copy_from_slice(cmd);
        self.send_request(delay)
    }

    /// Make a request. After making a request a [FutureResponse] must be created and consumed
    /// before making any new requests. This method is usually called through the API methods like
    /// `[card]`.
    pub(crate) fn request<T: Serialize>(
        &mut self,
        delay: &mut impl DelayMs<u16>,
        cmd: T,
    ) -> Result<(), NoteError> {
        self.buf.clear();
        self.buf.resize(self.buf.capacity(), 0).unwrap(); // unsafe { set_len } ?

        let sz = serde_json_core::to_slice(&cmd, &mut self.buf).map_err(|_| NoteError::SerError)?;
        self.buf.truncate(sz);

        // Add new-line, this separator tells the Notecard that the request is done.
        self.buf.push(b'\n').map_err(|_| NoteError::SerError)?;
        self.send_request(delay)
    }

    /// [card Requests](https://dev.blues.io/reference/notecard-api/card-requests/)
    pub fn card(&mut self) -> card::Card<IOM, BUF_SIZE> {
        card::Card::from(self)
    }

    /// [note Requests](https://dev.blues.io/reference/notecard-api/note-requests/)
    pub fn note(&mut self) -> note::Note<IOM, BUF_SIZE> {
        note::Note::from(self)
    }

    /// [web Requests](https://dev.blues.io/reference/notecard-api/web-requests/)
    pub fn web(&mut self) -> web::Web<IOM, BUF_SIZE> {
        web::Web::from(self)
    }

    /// [hub Requests](https://dev.blues.io/reference/notecard-api/hub-requests/)
    pub fn hub(&mut self) -> hub::Hub<IOM, BUF_SIZE> {
        hub::Hub::from(self)
    }

    /// [dfu Requests](https://dev.blues.io/api-reference/notecard-api/dfu-requests/)
    pub fn dfu(&mut self) -> dfu::DFU<IOM, BUF_SIZE> {
        dfu::DFU::from(self)
    }
}

/// A future response.
///
/// It will not be possible to make any new requests before this has been consumed. If you drop
/// this future before consuming the response the Notecard and driver will be left in inconsistent
/// state. It is not safe to make new requests to the Notecard before the previous response has
/// been read.
#[must_use = "The response must be waited for and consumed, otherwise the notecard is left in an inconsistent state"]
pub struct FutureResponse<
    'a,
    T: DeserializeOwned,
    IOM: Write<SevenBitAddress> + Read<SevenBitAddress>,
    const BUF_SIZE: usize,
> {
    note: &'a mut Notecard<IOM, BUF_SIZE>,
    _r: PhantomData<T>,
}

impl<
        'a,
        T: DeserializeOwned,
        IOM: Write<SevenBitAddress> + Read<SevenBitAddress>,
        const BUF_SIZE: usize,
    > FutureResponse<'a, T, IOM, BUF_SIZE>
{
    fn from(note: &'a mut Notecard<IOM, BUF_SIZE>) -> FutureResponse<'a, T, IOM, BUF_SIZE> {
        FutureResponse {
            note,
            _r: PhantomData,
        }
    }

    /// Reads remaining data and returns the deserialized object if it is ready.
    pub fn poll(&mut self) -> Result<Option<T>, NoteError> {
        match self.note.poll()? {
            Some(body) if body.starts_with(br##"{"err":"##) => {
                debug!(
                    "response is error response, parsing error..: {}",
                    core::str::from_utf8(&body).unwrap_or("[invalid utf-8]")
                );
                Err(
                    serde_json_core::from_slice::<NotecardError>(body).map_or_else(
                        |_| {
                            error!(
                                "failed to deserialize: {}",
                                core::str::from_utf8(&body).unwrap_or("[invalid utf-8]")
                            );
                            NoteError::new_desererror(&body)
                        },
                        |(e, _)| NoteError::from(e),
                    ),
                )
            }
            Some(body) => {
                trace!("response is regular, parsing..");
                Ok(Some(
                    serde_json_core::from_slice::<T>(body)
                        .map_err(|_| {
                            error!(
                                "failed to deserialize: {}",
                                core::str::from_utf8(&body).unwrap_or("[invalid utf-8]")
                            );
                            NoteError::new_desererror(&body)
                        })?
                        .0,
                ))
            }
            None => Ok(None),
        }
    }

    /// Wait for response and return raw bytes. These may change on next response,
    /// so this method is probably not staying as it is.
    pub fn wait_raw(mut self, delay: &mut impl DelayMs<u16>) -> Result<&'a [u8], NoteError> {
        let mut waited = 0;

        while waited < self.note.response_timeout {
            match self.poll()? {
                Some(_) => return Ok(self.note.take_response()?),
                None => (),
            }

            delay.delay_ms(RESPONSE_DELAY);
            waited += RESPONSE_DELAY;
        }

        error!("response timed out (>= {}).", self.note.response_timeout);
        Err(NoteError::TimeOut)
    }

    /// Wait for response and return deserialized object.
    pub fn wait(mut self, delay: &mut impl DelayMs<u16>) -> Result<T, NoteError> {
        let mut waited = 0;

        while waited < self.note.response_timeout {
            match self.poll()? {
                Some(r) => return Ok(r),
                None => (),
            }

            delay.delay_ms(RESPONSE_DELAY);
            waited += RESPONSE_DELAY;
        }

        error!("response timed out (>= {}).", self.note.response_timeout);
        Err(NoteError::TimeOut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal_mock::eh0::i2c::Mock;

    pub fn new_mock() -> Notecard<Mock> {
        // let exp = [ Transaction::write(0x17, vec![]) ];
        let i2c = Mock::new(&[]);
        Notecard::new(i2c)
    }

    #[test]
    fn resize_buf() {
        let c = new_mock();
        assert_eq!(c.buf.capacity(), DEFAULT_BUF_SIZE);

        let mut c = c.resize_buf::<1024>().unwrap();
        assert_eq!(c.buf.capacity(), 1024);

        c.i2c.done();
    }
}
