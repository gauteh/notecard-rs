//! https://dev.blues.io/reference/notecard-api/card-requests/

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use embedded_hal::blocking::delay::DelayMs;
use serde::{Deserialize, Serialize};

use super::{FutureResponse, NoteError, Notecard};

pub struct Card<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> {
    note: &'a mut Notecard<IOM, BS>,
}

impl<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> Card<'a, IOM, BS> {
    pub fn from(note: &mut Notecard<IOM, BS>) -> Card<'_, IOM, BS> {
        Card { note }
    }

    /// Retrieves current date and time information. Upon power-up, the Notecard must complete a
    /// sync to Notehub in order to obtain time and location data. Before the time is obtained,
    /// this request will return `{"zone":"UTC,Unknown"}`.
    pub fn time(self, delay: &mut impl DelayMs<u16>) -> Result<FutureResponse<'a, res::Time, IOM, BS>, NoteError> {
        self.note.request_raw(delay, b"{\"req\":\"card.time\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }

    /// Returns general information about the Notecard's operating status.
    pub fn status(self, delay: &mut impl DelayMs<u16>) -> Result<FutureResponse<'a, res::Status, IOM, BS>, NoteError> {
        self.note.request_raw(delay, b"{\"req\":\"card.status\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }

    /// Performs a firmware restart of the Notecard.
    pub fn restart(self, delay: &mut impl DelayMs<u16>) -> Result<FutureResponse<'a, res::Empty, IOM, BS>, NoteError> {
        self.note.request_raw(delay, b"{\"req\":\"card.restart\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }

    /// Retrieves the current location of the Notecard.
    pub fn location(self, delay: &mut impl DelayMs<u16>) -> Result<FutureResponse<'a, res::Location, IOM, BS>, NoteError> {
        self.note.request_raw(delay, b"{\"req\":\"card.location\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }

    /// Sets location-related configuration settings. Retrieves the current location mode when passed with no argument.
    pub fn location_mode(
        self,
        delay: &mut impl DelayMs<u16>,
        mode: Option<&str>,
        seconds: Option<u32>,
        vseconds: Option<&str>,
        delete: Option<bool>,
        max: Option<u32>,
        lat: Option<f32>,
        lon: Option<f32>,
        minutes: Option<u32>,
    ) -> Result<FutureResponse<'a, res::LocationMode, IOM, BS>, NoteError> {
        self.note.request(delay, req::LocationMode {
            req: "card.location.mode",
            mode: mode.map(heapless::String::from),
            seconds,
            vseconds: vseconds.map(heapless::String::from),
            delete,
            max,
            lat,
            lon,
            minutes,
        })?;
        Ok(FutureResponse::from(self.note))
    }

    pub fn location_track(
        self,
        delay: &mut impl DelayMs<u16>,
        start: bool,
        heartbeat: bool,
        sync: bool,
        hours: Option<i32>,
        file: Option<&str>,
    ) -> Result<FutureResponse<'a, res::LocationTrack, IOM, BS>, NoteError> {
        self.note.request(delay, req::LocationTrack {
            req: "card.location.track",
            start: start.then(|| true),
            stop: (!start).then(|| true),
            heartbeat: heartbeat.then(|| true),
            sync: sync.then(|| true),
            hours,
            file: file.map(heapless::String::from),
        })?;

        Ok(FutureResponse::from(self.note))
    }

    pub fn wireless(self, delay: &mut impl DelayMs<u16>) -> Result<FutureResponse<'a, res::Wireless, IOM, BS>, NoteError> {
        self.note.request_raw(delay, b"{\"req\":\"card.wireless\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }

    /// Returns firmware version information for the Notecard.
    pub fn version(self, delay: &mut impl DelayMs<u16>) -> Result<FutureResponse<'a, res::Version, IOM, BS>, NoteError> {
        self.note.request_raw(delay, b"{\"req\":\"card.version\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }
}

pub mod req {
    use super::*;

    #[derive(Deserialize, Serialize, defmt::Format, Default)]
    pub struct LocationTrack {
        pub req: &'static str,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub start: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub heartbeat: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub sync: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub stop: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub hours: Option<i32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub file: Option<heapless::String<20>>,
    }

    #[derive(Deserialize, Serialize, defmt::Format, Default)]
    pub struct LocationMode {
        pub req: &'static str,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub mode: Option<heapless::String<20>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub seconds: Option<u32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub vseconds: Option<heapless::String<20>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub delete: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub max: Option<u32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub lat: Option<f32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub lon: Option<f32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub minutes: Option<u32>,
    }
}

pub mod res {
    use super::*;

    #[derive(Deserialize, defmt::Format)]
    pub struct Empty {}

    #[derive(Deserialize, defmt::Format)]
    pub struct LocationTrack {
        pub start: Option<bool>,
        pub stop: Option<bool>,
        pub heartbeat: Option<bool>,
        pub seconds: Option<u32>,
        pub hours: Option<i32>,
        pub file: Option<heapless::String<20>>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct LocationMode {
        pub mode: heapless::String<20>,
        pub seconds: Option<u32>,
        pub vseconds: Option<heapless::String<20>>,
        pub max: Option<u32>,
        pub lat: Option<f64>,
        pub lon: Option<f64>,
        pub minutes: Option<u32>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Location {
        pub status: heapless::String<120>,
        pub mode: heapless::String<20>,
        pub lat: Option<f64>,
        pub lon: Option<f64>,
        pub time: Option<u32>,
        pub max: Option<u32>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Time {
        pub time: Option<u32>,
        pub area: Option<heapless::String<20>>,
        pub zone: Option<heapless::String<20>>,
        pub minutes: Option<i32>,
        pub lat: Option<f64>,
        pub lon: Option<f64>,
        pub country: Option<heapless::String<10>>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Status {
        pub status: heapless::String<10>,
        #[serde(default)]
        pub usb: bool,
        pub storage: usize,
        pub time: Option<u64>,
        #[serde(default)]
        pub connected: bool,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct WirelessNet {
        iccid: Option<heapless::String<24>>,
        imsi: Option<heapless::String<24>>,
        imei: Option<heapless::String<24>>,
        modem: Option<heapless::String<35>>,
        band: Option<heapless::String<24>>,
        rat: Option<heapless::String<24>>,
        rssir: Option<i32>,
        rssi: Option<i32>,
        rsrp: Option<i32>,
        sinr: Option<i32>,
        rsrq: Option<i32>,
        bars: Option<i32>,
        mcc: Option<i32>,
        mnc: Option<i32>,
        lac: Option<i32>,
        cid: Option<i32>,
        updated: Option<u32>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Wireless {
        pub status: heapless::String<24>,
        pub mode: Option<heapless::String<24>>,
        pub count: Option<u8>,
        pub net: Option<WirelessNet>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct VersionInner {
        pub org: heapless::String<24>,
        pub product: heapless::String<24>,
        pub version: heapless::String<24>,
        pub ver_major: u8,
        pub ver_minor: u8,
        pub ver_patch: u8,
        pub ver_build: u32,
        pub built: heapless::String<24>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Version {
        pub body: VersionInner,
        pub version: heapless::String<24>,
        pub device: heapless::String<24>,
        pub name: heapless::String<24>,
        pub board: heapless::String<24>,
        pub sku: heapless::String<24>,
        pub api: u16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NotecardError;

    #[test]
    fn test_version() {
        let r = br##"{
  "body": {
    "org":       "Blues Wireless",
    "product":   "Notecard",
    "version":   "notecard-1.5.0",
    "ver_major": 1,
    "ver_minor": 5,
    "ver_patch": 0,
    "ver_build": 11236,
    "built":     "Sep 2 2020 08:45:10"
  },
  "version": "notecard-1.5.0.11236",
  "device":  "dev:000000000000000",
  "name":    "Blues Wireless Notecard",
  "board":   "1.11",
  "sku":     "NOTE-WBNA500",
  "api":     1
}"##;
        serde_json_core::from_slice::<res::Version>(r).unwrap();
    }

    #[test]
    fn test_version_411() {
        let r = br##"{"version":"notecard-4.1.1.4015681","device":"dev:000000000000000","name":"Blues Wireless Notecard","sku":"NOTE-WBEX-500","board":"1.11","api":4,"body":{"org":"Blues Wireless","product":"Notecard","version":"notecard-4.1.1","ver_major":4,"ver_minor":1,"ver_patch":1,"ver_build":4015681,"built":"Dec  5 2022 12:54:58"}}"##;
        serde_json_core::from_slice::<res::Version>(r).unwrap();
    }

    #[test]
    fn test_card_wireless() {
        let r = br##"{"status":"{modem-on}","count":3,"net":{"iccid":"89011703278520607527","imsi":"310170852060752","imei":"864475044204278","modem":"BG95M3LAR02A03_01.006.01.006","band":"GSM 900","rat":"gsm","rssir":-77,"rssi":-77,"bars":3,"mcc":242,"mnc":1,"lac":11001,"cid":12313,"updated":1643923524}}"##;
        serde_json_core::from_slice::<res::Wireless>(r).unwrap();

        let r = br##"{"status":"{cell-registration-wait}","net":{"iccid":"89011703278520606586","imsi":"310170852060658","imei":"864475044197092","modem":"BG95M3LAR02A03_01.006.01.006"}}"##;
        serde_json_core::from_slice::<res::Wireless>(r).unwrap();

        let r = br##"{"status":"{modem-off}","net":{}}"##;
        serde_json_core::from_slice::<res::Wireless>(r).unwrap();

        let r = br##"{"status":"{network-up}","mode":"auto","count":3,"net":{"iccid":"89011703278520578660","imsi":"310170852057866","imei":"867730051260788","modem":"BG95M3LAR02A03_01.006.01.006","band":"GSM 900","rat":"gsm","rssir":-77,"rssi":-78,"bars":3,"mcc":242,"mnc":1,"lac":11,"cid":12286,"updated":1646227929}}"##;
        serde_json_core::from_slice::<res::Wireless>(r).unwrap();
    }

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

        serde_json_core::from_slice::<res::Time>(r).unwrap();
    }

    #[test]
    fn test_card_time_err() {
        let r = br##"{"err":"time is not yet set","zone":"UTC,Unknown"}"##;
        serde_json_core::from_slice::<NotecardError>(r).unwrap();
    }

    #[test]
    pub fn test_status_ok() {
        serde_json_core::from_str::<res::Status>(
            r#"
          {
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
        serde_json_core::from_str::<res::Status>(
            r#"
          {
            "status":    "{normal}",
            "usb":       true,
            "storage":   8
          }"#,
        )
        .unwrap();
    }

    #[test]
    fn test_partial_location_mode() {
        serde_json_core::from_str::<res::LocationMode>(r#"{"seconds":60,"mode":"periodic"}"#)
            .unwrap();
    }

    #[test]
    fn test_parse_exceed_string_size() {
        serde_json_core::from_str::<res::LocationMode>(r#"{"seconds":60,"mode":"periodicperiodicperiodicperiodicperiodicperiodicperiodic"}"#).ok();
    }

    #[test]
    fn test_location_searching() {
        serde_json_core::from_str::<res::Location>(
            r#"{"status":"GPS search (111 sec, 32/33 dB SNR, 0/1 sats) {gps-active} {gps-signal} {gps-sats}","mode":"continuous"}"#).unwrap();
    }

    #[test]
    fn test_location_mode_err() {
        let r = br##"{"err":"seconds: field seconds: unmarshal: expected a int32 {io}"}"##;
        serde_json_core::from_slice::<NotecardError>(r).unwrap();
    }
}
