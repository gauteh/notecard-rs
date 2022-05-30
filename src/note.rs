//! <https://dev.blues.io/reference/notecard-api/note-requests/>

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{FutureResponse, NoteError, Notecard};

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
    ///
    /// The size of the payload seems to be 250 bytes maximum.
    pub fn add<T: Serialize + Default>(
        self,
        delay: &mut impl DelayMs<u16>,
        file: Option<&str>,
        note: Option<&str>,
        body: Option<T>,
        payload: Option<&str>,
        sync: bool,
    ) -> Result<FutureResponse<'a, res::Add, IOM>, NoteError> {
        self.note.request(
            delay,
            req::Add::<T> {
                req: "note.add",
                file: file.map(heapless::String::from),
                note: note.map(heapless::String::from),
                body,
                payload,
                sync: Some(sync),
                ..Default::default()
            },
        )?;
        Ok(FutureResponse::from(self.note))
    }

    /// Updates a Note in a DB Notefile by its ID, replacing the existing body and/or payload.
    pub fn update<T: Serialize + Default>(
        self,
        delay: &mut impl DelayMs<u16>,
        file: &str,
        note: &str,
        body: Option<T>,
        payload: Option<&str>,
        verify: bool,
    ) -> Result<FutureResponse<'a, res::Empty, IOM>, NoteError> {
        self.note.request(
            delay,
            req::Update::<T> {
                req: "note.update",
                file: heapless::String::from(file),
                note: heapless::String::from(note),
                body,
                payload,
                verify,
            },
        )?;
        Ok(FutureResponse::from(self.note))
    }

    /// Retrieves a Note from a Notefile.
    ///
    /// * When sending this request to the Notecard, the file must either be a DB Notefile (.db or .dbx) or inbound queue file (.qi/.qis).
    /// * When sending this request to Notehub, the file must be a DB Notefile (.db).
    ///
    /// .qo/.qos Notes must be read from the Notehub event table using the Notehub Event API.
    pub fn get<T: DeserializeOwned + Serialize>(
        self,
        delay: &mut impl DelayMs<u16>,
        file: &str,
        note: &str,
        delete: bool,
        deleted: bool,
    ) -> Result<FutureResponse<'a, res::Get<T>, IOM>, NoteError> {
        self.note.request(
            delay,
            req::Get {
                req: "note.get",
                file: heapless::String::from(file),
                note: heapless::String::from(note),
                delete,
                deleted,
            },
        )?;
        Ok(FutureResponse::from(self.note))
    }

    /// Using the note.template request command with any .qo/.qos Notefile, developers can provide
    /// the Notecard with a schema of sorts to apply to future Notes added to the Notefile. This
    /// template acts as a hint to the Notecard that allows it to internally store data as
    /// fixed-length binary records rather than as flexible JSON objects which require much more
    /// memory. Using templated Notes in place of regular Notes increases the storage and sync
    /// capability of the Notecard by an order of magnitude.
    ///
    /// See
    /// https://dev.blues.io/notecard/notecard-walkthrough/low-bandwidth-design/#understanding-template-data-types
    /// for the format and values of the template.
    pub fn template<T: Serialize + Default>(
        self,
        delay: &mut impl DelayMs<u16>,
        file: Option<&str>,
        body: Option<T>,
        length: Option<u32>,
    ) -> Result<FutureResponse<'a, res::Template, IOM>, NoteError> {
        self.note.request(
            delay,
            req::Template::<T> {
                req: "note.template",
                file: file.map(heapless::String::from),
                body,
                length,
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

    #[derive(Deserialize, Serialize, Default)]
    pub struct Update<'a, T: Serialize + Default> {
        pub req: &'static str,

        pub file: heapless::String<20>,
        pub note: heapless::String<20>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub body: Option<T>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub payload: Option<&'a str>,

        pub verify: bool,
    }

    #[derive(Deserialize, Serialize, Default)]
    pub struct Get {
        pub req: &'static str,

        pub file: heapless::String<20>,
        pub note: heapless::String<20>,

        pub delete: bool,
        pub deleted: bool,
    }

    #[derive(Deserialize, Serialize, Default)]
    pub struct Template<T: Serialize + Default> {
        pub req: &'static str,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub file: Option<heapless::String<20>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub body: Option<T>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub length: Option<u32>,
    }
}

pub mod res {
    use super::*;

    #[derive(Deserialize, defmt::Format)]
    pub struct Empty {}

    #[derive(Deserialize, defmt::Format)]
    pub struct Get<T: Serialize> {
        pub body: Option<T>,
        pub payload: Option<heapless::String<1024>>,
        pub time: u32,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Add {
        total: Option<u32>,
        template: Option<bool>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Template {
        bytes: u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BUF_SIZE;

    #[test]
    fn add_with_template() {
        let r = br##"{"template":true}"##;
        serde_json_core::from_slice::<res::Add>(r).unwrap();
    }

    #[test]
    fn add_payload() {
        pub const AXL_OUTN: usize = { 3 * 1024 } * 4 * 4 / 3 + 4;

        #[derive(serde::Serialize, Default)]
        pub struct AxlPacket {
            pub timestamp: u32,

            /// This is added to the payload of the note.
            #[serde(skip)]
            pub data: heapless::Vec<u16, { 3 * 1024 }>,
        }

        let p = AxlPacket {
            timestamp: 0,
            data: (0..3072)
                .map(|v| v as _)
                .collect::<heapless::Vec<_, { 3 * 1024 }>>(),
        };

        let mut b64 = [0u8; AXL_OUTN];

        let data = bytemuck::cast_slice(&p.data);
        let sz = base64::encode_config_slice(data, base64::STANDARD, &mut b64);

        let b64 = &b64[..sz];
        let b64 = core::str::from_utf8(&b64).unwrap();

        let add = req::Add {
            req: "note.add",
            file: Some("axl.qo".into()),
            note: Some("?".into()),
            body: Some(p),
            payload: Some(b64),
            sync: Some(false),
            ..Default::default()
        };

        let cmd = serde_json_core::to_vec::<_, { BUF_SIZE }>(&add).unwrap();

        println!("cmd size: {}", cmd.len());
    }
}
