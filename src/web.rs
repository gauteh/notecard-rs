//! <https://dev.blues.io/api-reference/notecard-api/web-requests>

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{str_string, FutureResponse, NoteError, Notecard};

pub struct Web<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> {
    note: &'a mut Notecard<IOM, BS>,
}

impl<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> Web<'a, IOM, BS> {
    pub fn from(note: &mut Notecard<IOM, BS>) -> Web<'_, IOM, BS> {
        Web { note }
    }

    /// Performs a simple HTTP or HTTPS POST request against an external endpoint, and returns the response to the Notecard.
    pub fn post<T: Serialize + Default>(
        self,
        delay: &mut impl DelayMs<u16>,
        file: Option<&str>,
        note: Option<&str>,
        body: Option<T>,
        payload: Option<&str>,
        sync: bool,
    ) -> Result<FutureResponse<'a, res::Add, IOM, BS>, NoteError> {
        self.note.request(
            delay,
            req::Add::<T> {
                req: "note.add",
                file: str_string(file)?,
                note: str_string(note)?,
                body,
                payload,
                sync: Some(sync),
                ..<req::Add<T> as Default>::default()
            },
        )?;
        Ok(FutureResponse::from(self.note))
    }
}

mod req {
    use super::*;

    #[derive(Deserialize, Serialize, Default)]
    pub struct Add<'a, T: Serialize + Default> {
        pub req: &'static str,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub file: Option<heapless::String<20>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub note: Option<heapless::String<20>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub body: Option<T>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub payload: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub sync: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub key: Option<heapless::String<20>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub verify: Option<bool>,
    }
}

pub mod res {
    use super::*;

    #[derive(Deserialize, defmt::Format)]
    pub struct Add {
        total: Option<u32>,
        template: Option<bool>,
    }
}
