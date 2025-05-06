//! https://dev.blues.io/api-reference/notecard-api/ntn-requests/

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use serde::{Deserialize, Serialize};

use super::{FutureResponse, NoteError, Notecard};

pub struct NTN<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> {
    note: &'a mut Notecard<IOM, BS>,
}

#[derive(Clone, Copy, defmt::Format)]
pub enum NtnSetGps {
    /// Use notecard gps on starnote as well
    Notecard,

    /// Use starnotes own gps (default).
    Starnote,
}

impl<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> NTN<'a, IOM, BS> {
    pub fn from(note: &mut Notecard<IOM, BS>) -> NTN<'_, IOM, BS> {
        NTN { note }
    }

    /// Once a Notecard is connected to a Starnote device, the presence of a physical Starnote is stored in a permanent configuration that is not affected by a card.restore request. This request clears this configuration and allows you to return to testing NTN mode over cellular or Wi-Fi.
    pub fn reset<const PS: usize>(
        self,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<FutureResponse<'a, res::Empty, IOM, BS>, NoteError> {
        self.note.request_raw(delay, b"{\"req\":\"ntn.reset\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }

    /// Gets and sets the background download status of MCU host or Notecard
    /// firmware updates.
    pub fn status(
        self,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<FutureResponse<'a, res::Status, IOM, BS>, NoteError> {
        self.note
            .request_raw(delay, b"{\"req\":\"ntn.status\"}\n")?;

        Ok(FutureResponse::from(self.note))
    }

    /// Determines whether a Notecard should override a paired Starnote's GPS/GNSS location with its own GPS/GNSS location. The paired Starnote uses its own GPS/GNSS location by default.
    pub fn gps(
        self,
        delay: &mut impl DelayMs<u16>,
        gps: Option<NtnSetGps>,
    ) -> Result<FutureResponse<'a, res::Gps, IOM, BS>, NoteError> {
        self.note.request(
            delay,
            req::Gps {
                req: "ntn.gps",
                on: gps.map(|g| matches!(g, NtnSetGps::Notecard)),
                off: gps.map(|g| matches!(g, NtnSetGps::Starnote)),
            },
        )?;
        Ok(FutureResponse::from(self.note))
    }
}

pub mod req {
    use super::*;

    #[derive(Deserialize, Serialize, defmt::Format, Default)]
    pub struct Gps {
        pub req: &'static str,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub on: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub off: Option<bool>,
    }
}

pub mod res {
    use super::*;

    #[derive(Deserialize, defmt::Format)]
    pub struct Empty {}

    #[derive(Deserialize, Serialize, defmt::Format, Default)]
    pub struct Gps {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub on: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub off: Option<bool>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Status {
        pub err: Option<heapless::String<120>>,
        pub status: Option<heapless::String<120>>,
    }
}

// #[cfg(test)]
// mod tests {
//  use super::*;
// }
