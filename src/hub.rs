//! <https://dev.blues.io/reference/notecard-api/hub-requests/>

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use serde::{Deserialize, Serialize};

use super::{FutureResponse, Note, NoteError};

pub struct Hub<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> {
    note: &'a mut Note<IOM>,
}

impl<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> Hub<'a, IOM> {
    pub fn from(note: &mut Note<IOM>) -> Hub<'_, IOM> {
        Hub { note }
    }

    /// The [hub.set](https://dev.blues.io/reference/notecard-api/hub-requests/#hub-set) request is
    /// the primary method for controlling the Notecard's Notehub connection and sync behavior.
    pub fn set(
        self,
        product: Option<&str>,
        host: Option<&str>,
        mode: Option<req::HubMode>,
    ) -> Result<FutureResponse<'a, res::Empty, IOM>, NoteError> {
        self.note.request(req::HubSet {
            product,
            host,
            mode,
        })?;
        Ok(FutureResponse::from(self.note))
    }
}

mod req {
    use super::*;

    #[derive(Deserialize, Serialize, defmt::Format)]
    #[serde(rename_all = "lowercase")]
    pub enum HubMode {
        Periodic,
        Continuous,
        Minimum,
        Off,
        DFU,
    }

    #[derive(Deserialize, Serialize, defmt::Format)]
    pub struct HubSet<'a> {
        pub product: Option<&'a str>,
        pub host: Option<&'a str>,
        pub mode: Option<HubMode>,
    }
}

pub mod res {
    use super::*;

    #[derive(Deserialize, defmt::Format)]
    pub struct Empty {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_empty() {
        serde_json_core::from_str::<res::Empty>(r#"{}"#).unwrap();
    }

    #[test]
    pub fn hub_set_some() {
        let hb = req::HubSet {
            product: Some("testprod"),
            host: Some("testhost"),
            mode: Some(req::HubMode::Periodic),
        };

        assert_eq!(
            &serde_json_core::to_string::<_, 1024>(&hb).unwrap(),
            r#"{"product":"testprod","host":"testhost","mode":"periodic"}"#
        );
    }
}
