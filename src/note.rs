//! <https://dev.blues.io/reference/notecard-api/note-requests/>

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use serde::{Deserialize, Serialize};

use super::{FutureResponse, Notecard, NoteError};

pub struct Note<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> {
    note: &'a mut Notecard<IOM>,
}

impl<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> Note<'a, IOM> {
    pub fn from(note: &mut Notecard<IOM>) -> Note<'_, IOM> {
        Note { note }
    }

    /// Adds a note to a notefile, creating the Notefile if it doesn't yet exist. When sending this
    /// request to the Notecard, if a Notefile name is specified, the file must either be a DB
    /// Notefile or outbound queue file (.qo/.qos). When sending this request to Notehub, the file
    /// must either be a DB Notefile or an inbound queue file (.qi/.qis).
    pub fn add<T: Serialize + defmt::Format + Default>(self,
        file: Option<&str>,
        note: Option<&str>,
        body: Option<T>,
        payload: Option<&str>,
        sync: bool,
        ) -> Result<FutureResponse<'a, res::Add, IOM>, NoteError> {
        self.note.request(req::Add::<T> {
            req: "note.add",
            file: file.map(heapless::String::from),
            note: note.map(heapless::String::from),
            body,
            payload,
            sync: Some(sync),
            .. Default::default()
        })?;
        Ok(FutureResponse::from(self.note))
    }
}

mod req {
    use super::*;

    #[derive(Deserialize, Serialize, defmt::Format, Default)]
    pub struct Add<'a, T: Serialize + defmt::Format + Default> {
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
        pub verify: Option<bool>
    }
}

pub mod res {
    use super::*;

    #[derive(Deserialize, defmt::Format)]
    pub struct Add {
        total: u32,
        template: Option<bool>
    }
}

