//! <https://dev.blues.io/reference/notecard-api/hub-requests/>

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use serde::{Deserialize, Serialize};

use super::{FutureResponse, NoteError, Notecard};

pub struct Hub<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> {
    note: &'a mut Notecard<IOM, BS>,
}

impl<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> Hub<'a, IOM, BS> {
    pub fn from(note: &mut Notecard<IOM, BS>) -> Hub<'_, IOM, BS> {
        Hub { note }
    }

    /// Add a "device health" log message to send to Notehub on the next sync.
    pub fn log(
        self,
        delay: &mut impl DelayMs<u16>,
        text: &str,
        alert: bool,
        sync: bool,
    ) -> Result<FutureResponse<'a, res::Empty, IOM, BS>, NoteError> {
        self.note.request(
            delay,
            req::HubLog {
                req: "hub.log",
                text,
                alert,
                sync,
            },
        )?;
        Ok(FutureResponse::from(self.note))
    }

    /// The [hub.get](https://dev.blues.io/api-reference/notecard-api/hub-requests/#hub-get) request
    /// retrieves the current Notehub configuration for the Natecard.
    pub fn get(self, delay: &mut impl DelayMs<u16>) -> Result<FutureResponse<'a, res::Hub, IOM, BS>, NoteError> {
        self.note.request_raw(delay, b"{\"req\":\"hub.get\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }

    /// The [hub.set](https://dev.blues.io/reference/notecard-api/hub-requests/#hub-set) request is
    /// the primary method for controlling the Notecard's Notehub connection and sync behavior.
    pub fn set(
        self,
        delay: &mut impl DelayMs<u16>,
        product: Option<&str>,
        host: Option<&str>,
        mode: Option<req::HubMode>,
        sn: Option<&str>,
        outbound: Option<u32>,
        duration: Option<u32>,
        voutbound: Option<&str>,
        inbound: Option<u32>,
        vinbound: Option<&str>,
        align: Option<bool>,
        sync: Option<bool>,
    ) -> Result<FutureResponse<'a, res::Empty, IOM, BS>, NoteError> {
        self.note.request(
            delay,
            req::HubSet {
                req: "hub.set",
                product,
                host,
                mode,
                sn,
                outbound,
                duration,
                voutbound,
                inbound,
                vinbound,
                align,
                sync,
            },
        )?;
        Ok(FutureResponse::from(self.note))
    }

    /// Manually initiates a sync with Notehub. `allow` can be specified to `true` to
    /// remove the notecard from any penalty boxes.
    pub fn sync(
        self,
        delay: &mut impl DelayMs<u16>,
        allow: bool,
    ) -> Result<FutureResponse<'a, res::Empty, IOM, BS>, NoteError> {
        self.note.request(delay, req::HubSync {
            req: "hub.sync",
            allow: if allow { Some(true) } else { None }
        })?;

        Ok(FutureResponse::from(self.note))
    }

    /// Check on the status of a recently triggered or previous sync.
    pub fn sync_status(
        self,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<FutureResponse<'a, res::SyncStatus, IOM, BS>, NoteError> {
        self.note
            .request_raw(delay, b"{\"req\":\"hub.sync.status\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }
}

pub mod req {
    use super::*;

    #[derive(Deserialize, Serialize, defmt::Format, Default)]
    pub struct HubSync {
        pub req: &'static str,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub allow: Option<bool>,
    }

    #[derive(Deserialize, Serialize, defmt::Format)]
    #[serde(rename_all = "lowercase")]
    pub enum HubMode {
        Periodic,
        Continuous,
        Minimum,
        Off,
        DFU,
    }

    #[derive(Deserialize, Serialize, defmt::Format, Default)]
    pub struct HubSet<'a> {
        pub req: &'static str,

        pub product: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub host: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub mode: Option<HubMode>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub sn: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub outbound: Option<u32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub duration: Option<u32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub voutbound: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub inbound: Option<u32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub vinbound: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub align: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub sync: Option<bool>,
    }

    #[derive(Deserialize, Serialize, defmt::Format, Default)]
    pub struct HubLog<'a> {
        pub req: &'static str,
        pub text: &'a str,
        pub alert: bool,
        pub sync: bool,
    }
}

pub mod res {
    use super::*;

    #[derive(Deserialize, defmt::Format)]
    pub struct Empty {}

    #[derive(Deserialize, defmt::Format)]
    pub struct Hub {
        pub device: Option<heapless::String<40>>,
        pub product: Option<heapless::String<120>>,
        pub mode: Option<self::req::HubMode>,
        pub outbound: Option<u32>,
        pub voutbound: Option<f32>,
        pub inbound: Option<u32>,
        pub vinbound: Option<f32>,
        pub host: Option<heapless::String<40>>,
        pub sn: Option<heapless::String<120>>,
        pub sync: Option<bool>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct SyncStatus {
        pub status: Option<heapless::String<1024>>,
        pub time: Option<u32>,
        pub sync: Option<bool>,
        pub completed: Option<u32>,
        pub requested: Option<u32>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_empty() {
        serde_json_core::from_str::<res::Empty>(r#"{}"#).unwrap();
    }

    #[test]
    fn hub_get() {
        let r = br##"{
    "device": "dev:000000000000000",
    "product": "testprod",
    "mode": "periodic",
    "outbound": 60,
    "inbound": 240,
    "host": "a.notefile.net",
    "sn": "test-serial"
}"##;
        serde_json_core::from_slice::<res::Hub>(r).unwrap();
    }

    #[test]
    pub fn hub_set_some() {
        let hb = req::HubSet {
            req: "hub.set",
            product: Some("testprod"),
            host: Some("testhost"),
            mode: Some(req::HubMode::Periodic),
            ..Default::default()
        };

        assert_eq!(
            &serde_json_core::to_string::<_, 1024>(&hb).unwrap(),
            r#"{"req":"hub.set","product":"testprod","host":"testhost","mode":"periodic"}"#
        );
    }
}
