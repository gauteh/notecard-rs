//! https://dev.blues.io/reference/notecard-api/card-requests/

#[allow(unused_imports)]
use defmt::{error, warn, info, debug, trace};
use embedded_hal::blocking::i2c::{Write, Read, SevenBitAddress};
use serde::Deserialize;

use super::{Note, NoteError, FutureResponse};

pub struct Card<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> {
    note: &'a mut Note<IOM>
}

impl<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> Card<'a, IOM> {
    pub fn from(note: &mut Note<IOM>) -> Card<'_, IOM> {
        Card { note }
    }

    /// Retrieves current date and time information. Upon power-up, the Notecard must complete a
    /// sync to Notehub in order to obtain time and location data. Before the time is obtained,
    /// this request will return `{"zone":"UTC,Unknown"}`.
    pub fn time(self) -> Result<FutureResponse<'a, TimeResponse, IOM>, NoteError> {
        self.note.request(b"{\"req\":\"card.time\"}\n")?;
        Ok(FutureResponse::<'a, TimeResponse, IOM>::from(self.note))
    }
}

#[derive(Deserialize, defmt::Format)]
pub struct TimeResponse {
    time: u32,
    area: heapless::String<20>,
    zone: heapless::String<20>,
    minutes: i32,
    lat: f32,
    lon: f32,
    country: heapless::String<10>
}


// use serde::Deserialize;
// #[derive(Deserialize)]
// pub struct Status {
//     status: heapless::String<10>,
//     usb: bool,
//     storage: usize,
//     time: u64,
//     connected: bool,
// }

// pub fn status() -> Result<Status, ()> {
//     serde_json_core::from_str(
//         r#"{
//     "status":    "{normal}",
//     "usb":       true,
//     "storage":   8,
//     "time":      1599684765,
//     "connected": "true"
//     }"#,
//     )
//     .map_err(|_| ())
//     .map(|(a, _)| a)
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_time_err() {
        let r = br##"{"err":"time is not yet set","zone":"UTC,Unknown"}"##;
        serde_json_core::from_slice::<TimeResponse>(r).unwrap();
    }
}
