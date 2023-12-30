//! https://dev.blues.io/api-reference/notecard-api/dfu-requests/

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use embedded_hal::blocking::delay::DelayMs;
use serde::{Deserialize, Serialize};

use super::{FutureResponse, NoteError, Notecard};

pub struct DFU<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> {
    note: &'a mut Notecard<IOM, BS>,
}

impl<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> DFU<'a, IOM, BS> {
    pub fn from(note: &mut Notecard<IOM, BS>) -> DFU<'_, IOM, BS> {
        DFU { note }
    }

    /// Retrieves downloaded firmware data from the Notecard.
    /// Note: this request is functional only when the Notecard has been set to
    /// dfu mode with a `hub.set`, `mode:dfu` request.
    pub fn get<const PS: usize>(self, delay: &mut impl DelayMs<u16>, length: usize, offset: Option<usize>) -> Result<FutureResponse<'a, res::Get<PS>, IOM, BS>, NoteError> {
        self.note.request(delay, req::Get {
            req: "dfu.get",
            length,
            offset,
        })?;

        Ok(FutureResponse::from(self.note))
    }

    /// Gets and sets the background download status of MCU host or Notecard
    /// firmware updates.
    pub fn status(
        self,
        delay: &mut impl DelayMs<u16>,
        name: Option<req::StatusName>,
        stop: Option<bool>,
        status: Option<&str>,
        version: Option<&str>,
        vvalue: Option<&str>, // This is not JSON :(
        on: Option<bool>,
        err: Option<&str>
    ) -> Result<FutureResponse<'a, res::Status, IOM, BS>, NoteError> {
        self.note.request(delay, req::Status::new(
            name,
            stop,
            status,
            version,
            vvalue,
            on,
            err
        ))?;

        Ok(FutureResponse::from(self.note))
    }
}

pub mod req {
    use super::*;

    #[derive(Serialize, Deserialize, defmt::Format, Default)]
    pub struct Get {
        pub req: &'static str,

        pub length: usize,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub offset: Option<usize>
    }


    #[derive(Serialize, Deserialize, defmt::Format, PartialEq, Debug)]
    #[serde(rename_all = "lowercase")]
    pub enum StatusName {
        User,
        Card
    }

    #[derive(Serialize, Deserialize, defmt::Format)]
    pub struct Version<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub org: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub product: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub firmware: Option<&'a str>,
        pub version: &'a str,
        pub ver_major: u32,
        pub ver_minor: u32,
        pub ver_patch: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub ver_build: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub built: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub builder: Option<&'a str>,
    }

    #[derive(Serialize, Deserialize, defmt::Format, Default)]
    pub struct Status<'a> {
        pub req: &'static str,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub name: Option<StatusName>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub stop: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub status: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub version: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub vvalue: Option<&'a str>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub on: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub off: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub err: Option<&'a str>,
    }

    impl Status<'_> {
        pub fn new<'a>(
            name: Option<req::StatusName>,
            stop: Option<bool>,
            status: Option<&'a str>,
            version: Option<&'a str>,
            vvalue: Option<&'a str>, // This is not JSON :(
            on: Option<bool>,
            err: Option<&'a str>
        ) -> Status<'a> {
            // The `on`/`off` parameters are exclusive
            // When on is `true` we set `on` to `Some(True)` and `off` to `None`.
            // When on is `false` we set `on` to `None` and `off` to `Some(True)`.
            // This way we are not sending the `on` and `off` parameters together.
            Status {
                req: "dfu.status",
                name,
                stop,
                status,
                version,
                vvalue,
                on: on.and_then(|v| v.then_some(true)),
                off: on.and_then(|v| (!v).then_some(true)),
                err,
            }
        }
    }
}

pub mod res {
    use super::*;

    #[derive(Deserialize, defmt::Format)]
    pub struct Get<const PS: usize> {
        pub payload: heapless::String<PS>
    }

    #[derive(Deserialize, defmt::Format, PartialEq, Debug)]
    #[serde(rename_all = "lowercase")]
    pub enum StatusMode {
        Idle,
        Error,
        Downloading,
        Ready
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct StatusBody {
        pub crc32: Option<u32>,
        pub created: Option<u32>,
        //pub info: JSON?
        pub length: Option<usize>,
        pub md5: Option<heapless::String<32>>,
        pub modified: Option<u32>,
        pub name: Option<heapless::String<120>>,
        pub notes: Option<heapless::String<120>>,
        pub source: Option<heapless::String<120>>,
        #[serde(rename = "type")]
        pub bin_type: Option<heapless::String<120>>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Status {
        pub mode: StatusMode,
        pub status: Option<heapless::String<120>>,
        pub on: Option<bool>,
        pub off: Option<bool>,
        pub pending: Option<bool>,
        pub body: Option<StatusBody>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get() {
        let (res, _) = serde_json_core::from_str::<res::Get<32>>(r#"{"payload":"THISISALOTOFBINARYDATA="}"#).unwrap();
        assert_eq!(res.payload, r#"THISISALOTOFBINARYDATA="#);
    }

    #[test]
    fn test_status_name() {
        let res: heapless::String<32> = serde_json_core::to_string(&req::StatusName::Card).unwrap();
        assert_eq!(res, r#""card""#);
        let res: heapless::String<32> = serde_json_core::to_string(&req::StatusName::User).unwrap();
        assert_eq!(res, r#""user""#);
    }

    #[test]
    fn test_status_req() {
        // Test basic request
        let req = req::Status::new(
            None,
            None,
            None,
            None,
            None,
            None,
            None
        );
        let res: heapless::String<256> = serde_json_core::to_string(&req).unwrap();
        assert_eq!(res, r#"{"req":"dfu.status"}"#);

        // Test a bunch of fields set
        let ver = req::Version {
            org: Some("Organization"),
            product: Some("Product"),
            description: Some("Firmware Description"),
            firmware: Some("Firmware Name"),
            version: "Firmware Version 1.0.0",
            ver_major: 1,
            ver_minor: 0,
            ver_patch: 0,
            ver_build: Some(12345),
            built: Some("Some Sunny Day In December"),
            builder: Some("The Compnay"),
        };
        let ver_str: heapless::String<512> = serde_json_core::to_string(&ver).unwrap();
        let req = req::Status::new(
            Some(req::StatusName::User),
            Some(true),
            Some("test status"),
            Some(ver_str.as_str()),
            Some("usb:1;high:1;normal:1;low:0;dead:0"),
            Some(true),
            Some("test error"),
        );
        let res: heapless::String<512> = serde_json_core::to_string(&req).unwrap();
        assert_eq!(res, r#"{"req":"dfu.status","name":"user","stop":true,"status":"test status","version":"{\"org\":\"Organization\",\"product\":\"Product\",\"description\":\"Firmware Description\",\"firmware\":\"Firmware Name\",\"version\":\"Firmware Version 1.0.0\",\"ver_major\":1,\"ver_minor\":0,\"ver_patch\":0,\"ver_build\":12345,\"built\":\"Some Sunny Day In December\",\"builder\":\"The Compnay\"}","vvalue":"usb:1;high:1;normal:1;low:0;dead:0","on":true,"err":"test error"}"#);

        // Test off set
        let req = req::Status::new(
            None,
            None,
            None,
            None,
            None,
            Some(false),
            None
        );
        let res: heapless::String<256> = serde_json_core::to_string(&req).unwrap();
        assert_eq!(res, r#"{"req":"dfu.status","off":true}"#);
    }

    #[test]
    fn test_status_mode() {
        let (res, _) = serde_json_core::from_str::<res::StatusMode>(r#""downloading""#).unwrap();
        assert_eq!(res, res::StatusMode::Downloading);
        let (res, _) = serde_json_core::from_str::<res::StatusMode>(r#""error""#).unwrap();
        assert_eq!(res, res::StatusMode::Error);
        let (res, _) = serde_json_core::from_str::<res::StatusMode>(r#""idle""#).unwrap();
        assert_eq!(res, res::StatusMode::Idle);
        let (res, _) = serde_json_core::from_str::<res::StatusMode>(r#""ready""#).unwrap();
        assert_eq!(res, res::StatusMode::Ready);
    }

    #[test]
    fn test_status() {
        let (res, _) = serde_json_core::from_str::<res::Status>(r#"{
            "mode": "ready",
            "status": "successfully downloaded",
            "on": true,
            "body": {
                "crc32": 2525287425,
                "created": 1599163431,
                "info": {},
                "length": 42892,
                "md5": "5a3f73a7f1b4bc8917b12b36c2532969",
                "modified": 1599163431,
                "name": "stm32-new-firmware$20200903200351.bin",
                "notes": "Latest prod firmware",
                "source": "stm32-new-firmware.bin",
                "type": "firmware"
            }
        }"#).unwrap();

        assert_eq!(res.mode, res::StatusMode::Ready);
        assert_eq!(res.status.unwrap(), "successfully downloaded");
        assert_eq!(res.on.unwrap(), true);
        let body = res.body.unwrap();
        assert_eq!(body.crc32.unwrap(), 2525287425);
        assert_eq!(body.created.unwrap(), 1599163431);
        assert_eq!(body.length.unwrap(), 42892);
        assert_eq!(body.md5.unwrap(), "5a3f73a7f1b4bc8917b12b36c2532969");
        assert_eq!(body.modified.unwrap(), 1599163431);
        assert_eq!(body.name.unwrap(), "stm32-new-firmware$20200903200351.bin");
        assert_eq!(body.notes.unwrap(), "Latest prod firmware");
        assert_eq!(body.source.unwrap(), "stm32-new-firmware.bin");
        assert_eq!(body.bin_type.unwrap(), "firmware");
    }
}