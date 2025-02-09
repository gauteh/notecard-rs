//! <https://dev.blues.io/api-reference/notecard-api/web-requests>

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::i2c::I2c;
use serde::{Deserialize, Serialize};

use super::{str_string, FutureResponse, NoteError, Notecard};

pub struct Web<'a, IOM: I2c, const BS: usize> {
    note: &'a mut Notecard<IOM, BS>,
}

impl<'a, IOM: I2c, const BS: usize> Web<'a, IOM, BS> {
    pub fn from(note: &mut Notecard<IOM, BS>) -> Web<'_, IOM, BS> {
        Web { note }
    }

    /// Performs a simple HTTP or HTTPS POST request against an external endpoint, and returns the response to the Notecard.
    pub async fn post<T: Serialize + Default>(
        self,
        delay: &mut impl DelayNs,
        route: &str,
        name: Option<&str>,
        body: Option<T>,
        payload: Option<&str>,
        content: Option<&str>,
        seconds: Option<u16>,
        max: Option<u16>,
        verify: Option<bool>,
        nasync: Option<bool>,
    ) -> Result<FutureResponse<'a, res::Post, IOM, BS>, NoteError> {
        self.note.request(
            delay,
            req::Post::<T> {
                req: "web.post",
                route: heapless::String::try_from(route).map_err(NoteError::string_err)?,
                name: str_string(name)?,
                body,
                payload,
                content: str_string(content)?,
                seconds,
                max,
                verify,
                nasync,
            },
        ).await?;
        Ok(FutureResponse::from(self.note))
    }
}

mod req {
    use super::*;

    #[derive(Deserialize, Serialize, Debug, defmt::Format, Default)]
    pub struct Post<'a, T: Serialize + Default> {
        pub req: &'static str,

        pub route: heapless::String<256>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub name: Option<heapless::String<256>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub body: Option<T>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub payload: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub content: Option<heapless::String<256>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub seconds: Option<u16>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub max: Option<u16>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub verify: Option<bool>,

        #[serde(rename = "async", skip_serializing_if = "Option::is_none")]
        pub nasync: Option<bool>,
    }
}

pub mod res {
    use super::*;

    #[derive(Deserialize, Debug, defmt::Format)]
    pub struct Post {
        result: Option<u32>,
        // body: Option<&'a str>,
        // payload: Option<&'a str>,
        // status: Option<&'a str>,
        // cobs: Option<u32>,
        // length: Option<u32>,
    }
}
