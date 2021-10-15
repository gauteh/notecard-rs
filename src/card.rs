//! https://dev.blues.io/reference/notecard-api/card-requests/

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use serde::Deserialize;

use super::{FutureResponse, Note, NoteError};

pub struct Card<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> {
    note: &'a mut Note<IOM>,
}

impl<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> Card<'a, IOM> {
    pub fn from(note: &mut Note<IOM>) -> Card<'_, IOM> {
        Card { note }
    }

    /// Retrieves current date and time information. Upon power-up, the Notecard must complete a
    /// sync to Notehub in order to obtain time and location data. Before the time is obtained,
    /// this request will return `{"zone":"UTC,Unknown"}`.
    pub fn time(self) -> Result<FutureResponse<'a, Time, IOM>, NoteError> {
        self.note.request(b"{\"req\":\"card.time\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }

    /// Returns general information about the Notecard's operating status.
    pub fn status(self) -> Result<FutureResponse<'a, Status, IOM>, NoteError> {
        self.note.request(b"{\"req\":\"card.status\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }
}

#[derive(Deserialize, defmt::Format)]
pub struct Time {
    time: u32,
    area: heapless::String<20>,
    zone: heapless::String<20>,
    minutes: i32,
    lat: f32,
    lon: f32,
    country: heapless::String<10>,
}

#[derive(Deserialize, defmt::Format)]
pub struct Status {
    status: heapless::String<10>,
    usb: bool,
    storage: usize,

    time: Option<u64>,

    #[serde(default)]
    connected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NotecardError;

    #[test]
    fn test_card_time_ok() {
        let r = br##"
{
  "time": 1599769214,
  "area": "Beverly, MA",
  "zone": "CDT,America/New York",
  "minutes": -300,
  "lat": 42.5776,
  "lon": -70.87134,
  "country": "US"
}
"##;
        serde_json_core::from_slice::<Time>(r).unwrap();
    }

    #[test]
    fn test_card_time_err() {
        let r = br##"{"err":"time is not yet set","zone":"UTC,Unknown"}"##;
        serde_json_core::from_slice::<NotecardError>(r).unwrap();
    }

    #[test]
    pub fn test_status_ok() {
        serde_json_core::from_str::<Status>(
            r#"{
    "status":    "{normal}",
    "usb":       true,
    "storage":   8,
    "time":      1599684765,
    "connected": true
    }"#,
        )
        .unwrap();
    }

    #[test]
    pub fn test_status_mising() {
        serde_json_core::from_str::<Status>(
            r#"{
    "status":    "{normal}",
    "usb":       true,
    "storage":   8
    }"#,
        )
        .unwrap();
    }
}
