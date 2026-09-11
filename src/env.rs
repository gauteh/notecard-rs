//! <https://dev.blues.io/reference/notecard-api/env-requests/>
//!
//! Environment variables are set on [Notehub](https://notehub.io) (or locally
//! on the Notecard) and pushed down to the device, independent of firmware
//! updates. This is the mechanism used for remote, live reconfiguration of
//! deployed devices (e.g. changing sync/duty-cycle behavior without
//! reflashing).

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use serde::{Deserialize, Serialize};

use super::{FutureResponse, NoteError, Notecard};

pub struct Env<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> {
    note: &'a mut Notecard<IOM, BS>,
}

impl<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> Env<'a, IOM, BS> {
    pub fn from(note: &mut Notecard<IOM, BS>) -> Env<'_, IOM, BS> {
        Env { note }
    }

    /// The [env.get](https://dev.blues.io/reference/notecard-api/env-requests/#env-get) request
    /// retrieves the current value of a single named environment variable. Retrieving a single,
    /// known variable name (rather than all variables at once) avoids needing to deserialize an
    /// arbitrary/unbounded JSON object on this `no_std` target.
    pub fn get(
        self,
        delay: &mut impl DelayMs<u16>,
        name: &str,
    ) -> Result<FutureResponse<'a, res::Env, IOM, BS>, NoteError> {
        self.note.request(
            delay,
            req::EnvGet {
                req: "env.get",
                name: Some(name),
            },
        )?;
        Ok(FutureResponse::from(self.note))
    }

    /// The [env.modified](https://dev.blues.io/reference/notecard-api/env-requests/#env-modified)
    /// request returns only the last-modified time of the environment variables (no variable
    /// values), and is much cheaper than [`Env::get`] — useful for cheaply polling whether
    /// anything has changed before doing a full fetch.
    pub fn modified(
        self,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<FutureResponse<'a, res::EnvModified, IOM, BS>, NoteError> {
        self.note
            .request_raw(delay, b"{\"req\":\"env.modified\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }
}

pub mod req {
    use super::*;

    #[derive(Deserialize, Serialize, Debug, defmt::Format, Default)]
    pub struct EnvGet<'a> {
        pub req: &'static str,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub name: Option<&'a str>,
    }
}

pub mod res {
    use super::*;

    #[derive(Deserialize, Debug, defmt::Format, Default)]
    pub struct Env {
        /// Value of the requested environment variable. Absent if the variable is not set.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub text: Option<heapless::String<64>>,

        /// Unix time (seconds) the variable was last modified.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub time: Option<u32>,
    }

    #[derive(Deserialize, Debug, defmt::Format, Default)]
    pub struct EnvModified {
        /// Unix time (seconds) any environment variable was last modified.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub time: Option<u32>,
    }
}
