//! https://dev.blues.io/reference/notecard-api/card-requests/

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};
use serde::{Deserialize, Serialize};

use super::{str_string, FutureResponse, NoteError, Notecard};

pub struct Card<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> {
    note: &'a mut Notecard<IOM, BS>,
}

/// https://dev.blues.io/api-reference/notecard-api/card-requests/latest/#card-transport
pub enum Transport {
    Reset,
    WifiCell,
    Wifi,
    Cell,
    NTN,
    WifiNTN,
    CellNTN,
    WifiCellNTN,
}

impl Transport {
    pub fn str(&self) -> &'static str {
        use Transport::*;

        match self {
            Reset => "-",
            WifiCell => "wifi-cell",
            Wifi => "wifi",
            Cell => "cell",
            NTN => "ntn",
            WifiNTN => "wifi-ntn",
            CellNTN => "cell-ntn",
            WifiCellNTN => "wifi-cell-ntn",
        }
    }
}

impl<'a, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>, const BS: usize> Card<'a, IOM, BS> {
    pub fn from(note: &mut Notecard<IOM, BS>) -> Card<'_, IOM, BS> {
        Card { note }
    }

    /// Retrieves current date and time information. Upon power-up, the Notecard must complete a
    /// sync to Notehub in order to obtain time and location data. Before the time is obtained,
    /// this request will return `{"zone":"UTC,Unknown"}`.
    pub fn time(
        self,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<FutureResponse<'a, res::Time, IOM, BS>, NoteError> {
        self.note.request_raw(delay, b"{\"req\":\"card.time\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }

    /// Returns general information about the Notecard's operating status.
    pub fn status(
        self,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<FutureResponse<'a, res::Status, IOM, BS>, NoteError> {
        self.note
            .request_raw(delay, b"{\"req\":\"card.status\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }

    /// Performs a firmware restart of the Notecard.
    pub fn restart(
        self,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<FutureResponse<'a, res::Empty, IOM, BS>, NoteError> {
        self.note
            .request_raw(delay, b"{\"req\":\"card.restart\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }

    /// Retrieves the current location of the Notecard.
    pub fn location(
        self,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<FutureResponse<'a, res::Location, IOM, BS>, NoteError> {
        self.note
            .request_raw(delay, b"{\"req\":\"card.location\"}\n")?;
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
        self.note.request(
            delay,
            req::LocationMode {
                req: "card.location.mode",
                mode: str_string(mode)?,
                seconds,
                vseconds: str_string(vseconds)?,
                delete,
                max,
                lat,
                lon,
                minutes,
            },
        )?;
        Ok(FutureResponse::from(self.note))
    }

    /// Store location data in a Notefile at the `periodic` interval, or using specified `heartbeat`.
    /// Only available when `card.location.mode` has been set to `periodic`.
    pub fn location_track(
        self,
        delay: &mut impl DelayMs<u16>,
        start: bool,
        heartbeat: bool,
        sync: bool,
        hours: Option<i32>,
        file: Option<&str>,
    ) -> Result<FutureResponse<'a, res::LocationTrack, IOM, BS>, NoteError> {
        self.note.request(
            delay,
            req::LocationTrack {
                req: "card.location.track",
                start: start.then(|| true),
                stop: (!start).then(|| true),
                heartbeat: heartbeat.then(|| true),
                sync: sync.then(|| true),
                hours,
                file: str_string(file)?,
            },
        )?;

        Ok(FutureResponse::from(self.note))
    }

    pub fn wireless(
        self,
        delay: &mut impl DelayMs<u16>,
        mode: Option<&str>,
        apn: Option<&str>,
        method: Option<&str>,
        hours: Option<u32>,
    ) -> Result<FutureResponse<'a, res::Wireless, IOM, BS>, NoteError> {
        self.note.request(
            delay,
            req::Wireless {
                req: "card.wireless",
                mode: str_string(mode)?,
                method: str_string(method)?,
                apn: str_string(apn)?,
                hours,
            },
        )?;

        Ok(FutureResponse::from(self.note))
    }

    /// Returns firmware version information for the Notecard.
    pub fn version(
        self,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<FutureResponse<'a, res::Version, IOM, BS>, NoteError> {
        self.note
            .request_raw(delay, b"{\"req\":\"card.version\"}\n")?;
        Ok(FutureResponse::from(self.note))
    }

    /// Configure Notecard Outboard Firmware Update feature
    /// Added in v3.5.1 Notecard Firmware.
    pub fn dfu(
        self,
        delay: &mut impl DelayMs<u16>,
        name: Option<req::DFUName>,
        on: Option<bool>,
        stop: Option<bool>,
    ) -> Result<FutureResponse<'a, res::DFU, IOM, BS>, NoteError> {
        self.note.request(delay, req::DFU::new(name, on, stop))?;
        Ok(FutureResponse::from(self.note))
    }

    pub fn transport(
        self,
        delay: &mut impl DelayMs<u16>,
        method: Transport,
        allow: Option<bool>,
        umin: Option<bool>,
    ) -> Result<FutureResponse<'a, res::Transport, IOM, BS>, NoteError> {
        self.note.request(
            delay,
            req::Transport {
                req: "card.transport",
                method: method.str(),
                allow,
                umin,
            },
        )?;
        Ok(FutureResponse::from(self.note))
    }
}

pub mod req {
    use super::*;

    #[derive(Deserialize, Serialize, defmt::Format, Default)]
    pub struct Transport {
        pub req: &'static str,

        pub method: &'static str,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub allow: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub umin: Option<bool>,
    }

    #[derive(Deserialize, Serialize, defmt::Format, Default)]
    pub struct Wireless {
        pub req: &'static str,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub mode: Option<heapless::String<20>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub apn: Option<heapless::String<120>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub method: Option<heapless::String<120>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub hours: Option<u32>,
    }

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

    #[derive(Deserialize, Serialize, defmt::Format, PartialEq, Debug)]
    #[serde(rename_all = "lowercase")]
    pub enum DFUName {
        Esp32,
        Stm32,
        #[serde(rename = "stm32-bi")]
        Stm32Bi,
        McuBoot,
        #[serde(rename = "-")]
        Reset,
    }

    #[derive(Deserialize, Serialize, defmt::Format)]
    pub struct DFU {
        pub req: &'static str,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub name: Option<req::DFUName>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub on: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub off: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub stop: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub start: Option<bool>,
    }

    impl DFU {
        pub fn new(name: Option<req::DFUName>, on: Option<bool>, stop: Option<bool>) -> Self {
            // The `on`/`off` and `stop`/`start` parameters are exclusive
            // When on is `true` we set `on` to `Some(True)` and `off` to `None`.
            // When on is `false` we set `on` to `None` and `off` to `Some(True)`.
            // This way we are not sending the `on` and `off` parameters together.
            // Same thing applies to the `stop`/`start` parameter.
            Self {
                req: "card.dfu",
                name,
                on: on.and_then(|v| if v { Some(true) } else { None }),
                off: on.and_then(|v| if v { None } else { Some(true) }),
                stop: stop.and_then(|v| if v { Some(true) } else { None }),
                start: stop.and_then(|v| if v { None } else { Some(true) }),
            }
        }
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
        pub mode: heapless::String<60>,
        pub seconds: Option<u32>,
        pub vseconds: Option<heapless::String<40>>,
        pub max: Option<u32>,
        pub lat: Option<f64>,
        pub lon: Option<f64>,
        pub minutes: Option<u32>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Location {
        pub status: heapless::String<120>,
        pub mode: heapless::String<120>,
        pub lat: Option<f64>,
        pub lon: Option<f64>,
        pub time: Option<u32>,
        pub max: Option<u32>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Time {
        pub time: Option<u32>,
        pub area: Option<heapless::String<120>>,
        pub zone: Option<heapless::String<120>>,
        pub minutes: Option<i32>,
        pub lat: Option<f64>,
        pub lon: Option<f64>,
        pub country: Option<heapless::String<120>>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Status {
        pub status: heapless::String<40>,
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
        pub target: Option<heapless::String<5>>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Version {
        pub body: VersionInner,
        pub version: heapless::String<24>,
        pub device: heapless::String<24>,
        pub name: heapless::String<30>,
        pub board: heapless::String<24>,
        pub sku: heapless::String<24>,
        pub api: Option<u16>,
        pub cell: Option<bool>,
        pub gps: Option<bool>,
        pub ordering_code: Option<heapless::String<50>>,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct DFU {
        pub name: req::DFUName,
    }

    #[derive(Deserialize, defmt::Format)]
    pub struct Transport {
        pub method: heapless::String<120>,
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
    fn test_version_752() {
        let r = br##"{"version":"notecard-7.5.2.17004","device":"dev:861059067974133","name":"Blues Wireless Notecard","sku":"NOTE-NBGLN","ordering_code":"EB0WT1N0AXBA","board":"5.13","cell":true,"gps":true,"body":{"org":"Blues Wireless","product":"Notecard","target":"u5","version":"notecard-u5-7.5.2","ver_major":7,"ver_minor":5,"ver_patch":2,"ver_build":17004,"built":"Nov 26 2024 14:01:26"}}"##;
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
    fn test_card_time_sa() {
        let r = br##"
        {
          "time": 1599769214,
          "area": "Kommetjie Western Cape",
          "zone": "Africa/Johannesburg",
          "minutes": -300,
          "lat": 42.5776,
          "lon": -70.87134,
          "country": "ZA"
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
        serde_json_core::from_str::<res::LocationMode>(
            r#"{"seconds":60,"mode":"periodicperiodicperiodicperiodicperiodicperiodicperiodic"}"#,
        )
        .ok();
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

    #[test]
    fn test_dfu_name() {
        let (res, _) = serde_json_core::from_str::<req::DFUName>(r#""esp32""#).unwrap();
        assert_eq!(res, req::DFUName::Esp32);
        let (res, _) = serde_json_core::from_str::<req::DFUName>(r#""stm32""#).unwrap();
        assert_eq!(res, req::DFUName::Stm32);
        let (res, _) = serde_json_core::from_str::<req::DFUName>(r#""stm32-bi""#).unwrap();
        assert_eq!(res, req::DFUName::Stm32Bi);
        let (res, _) = serde_json_core::from_str::<req::DFUName>(r#""mcuboot""#).unwrap();
        assert_eq!(res, req::DFUName::McuBoot);
        let (res, _) = serde_json_core::from_str::<req::DFUName>(r#""-""#).unwrap();
        assert_eq!(res, req::DFUName::Reset);
    }

    #[test]
    fn test_dfu_req() {
        // Test basic request
        let req = req::DFU::new(None, None, None);
        let res: heapless::String<1024> = serde_json_core::to_string(&req).unwrap();
        assert_eq!(res, r#"{"req":"card.dfu"}"#);

        // Test name & on request
        let req = req::DFU::new(Some(req::DFUName::Esp32), Some(true), None);
        let res: heapless::String<256> = serde_json_core::to_string(&req).unwrap();
        assert_eq!(res, r#"{"req":"card.dfu","name":"esp32","on":true}"#);

        // Test off request
        let req = req::DFU::new(None, Some(false), None);
        let res: heapless::String<256> = serde_json_core::to_string(&req).unwrap();
        assert_eq!(res, r#"{"req":"card.dfu","off":true}"#);

        // Test stop request
        let req = req::DFU::new(None, None, Some(true));
        let res: heapless::String<256> = serde_json_core::to_string(&req).unwrap();
        assert_eq!(res, r#"{"req":"card.dfu","stop":true}"#);

        // Test start request
        let req = req::DFU::new(None, None, Some(false));
        let res: heapless::String<256> = serde_json_core::to_string(&req).unwrap();
        assert_eq!(res, r#"{"req":"card.dfu","start":true}"#);
    }

    #[test]
    fn test_dfu_res() {
        serde_json_core::from_str::<res::DFU>(r#"{"name": "stm32"}"#).unwrap();
    }
}
