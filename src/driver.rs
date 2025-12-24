// Thanks to https://github.com/smunaut/blackmagic-misc for reverse
// engineering the difficult parts like authentication!

use hidapi::{HidApi, HidDevice};

const VENDOR_ID: u16 = 0x1EDB; // Blackmagic Design
const PRODUCT_ID: u16 = 0xDA0E; // Speed Editor

#[derive(Debug, Clone, PartialEq)]
pub enum Report {
    Wheel { mode: WheelMode, value: i32 },
    Buttons(Vec<Button>),
    Battery { charging: bool, level: u8 },
}

impl TryFrom<&[u8]> for Report {
    type Error = crate::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.is_empty() {
            return Err(Self::Error::Driver { message: "" });
        }

        let report_id = bytes[0];

        let report = match report_id {
            0x03 => {
                if bytes.len() != 7 {
                    return Err(crate::Error::Driver {
                        message: "invalid length for wheel report",
                    });
                }
                Report::Wheel {
                    mode: WheelMode::try_from(bytes[1])?,
                    value: i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
                }
            }
            0x04 => {
                if bytes.len() != 13 {
                    return Err(crate::Error::Driver {
                        message: "invalid length for button report",
                    });
                }

                let mut buttons = Vec::new();
                for chunk in bytes[1..13].chunks(2) {
                    let val = u16::from_le_bytes([chunk[0], chunk[1]]);
                    if val != 0x00 {
                        buttons.push(Button::try_from(val)?);
                    }
                }

                Report::Buttons(buttons)
            }
            0x07 => {
                if bytes.len() != 3 {
                    return Err(crate::Error::Driver {
                        message: "invalid length for battery report",
                    });
                }

                Report::Battery { charging: bytes[1] == 0x01, level: bytes[2] }
            }
            _ => {
                return Err(crate::Error::Driver { message: "unknown report" });
            }
        };

        Ok(report)
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    SmartInsert = 0x0001,
    Append = 0x0002,
    RippleOverwrite = 0x0003,
    CloseUp = 0x0004,
    PlaceOnTop = 0x0005,
    SourceOverwrite = 0x0006,

    In = 0x0007,
    Out = 0x0008,
    TrimIn = 0x0009,
    TrimOut = 0x000a,
    Roll = 0x000b,
    SlipSource = 0x000c,
    SlipDestination = 0x000d,
    TransitionDuration = 0x000e,
    Cut = 0x000f,
    Dissolve = 0x0010,
    SmoothCut = 0x0011,

    Escape = 0x0031,
    SyncBin = 0x001f,
    AudioLevel = 0x002c,
    FullView = 0x002d,
    Transition = 0x0022,
    Split = 0x002f,
    Snap = 0x002e,
    RippleDelete = 0x002b,

    Cam1 = 0x0033,
    Cam2 = 0x0034,
    Cam3 = 0x0035,
    Cam4 = 0x0036,
    Cam5 = 0x0037,
    Cam6 = 0x0038,
    Cam7 = 0x0039,
    Cam8 = 0x003a,
    Cam9 = 0x003b,
    LiveOverwrite = 0x0030,
    VideoOnly = 0x0025,
    AudioOnly = 0x0026,
    StopPlay = 0x003c,

    Source = 0x001a,
    Timeline = 0x001b,

    Shuttle = 0x001c,
    Jog = 0x001d,
    Scroll = 0x001e,
}

impl Button {
    pub fn led(&self) -> Option<Led> {
        match self {
            Button::CloseUp => Some(Led::Button(ButtonLed::CloseUp)),
            Button::Cut => Some(Led::Button(ButtonLed::Cut)),
            Button::Dissolve => Some(Led::Button(ButtonLed::Dissolve)),
            Button::SmoothCut => Some(Led::Button(ButtonLed::SmoothCut)),
            Button::Transition => Some(Led::Button(ButtonLed::Transition)),
            Button::Snap => Some(Led::Button(ButtonLed::Snap)),
            Button::Cam7 => Some(Led::Button(ButtonLed::Cam7)),
            Button::Cam8 => Some(Led::Button(ButtonLed::Cam8)),
            Button::Cam9 => Some(Led::Button(ButtonLed::Cam9)),
            Button::LiveOverwrite => Some(Led::Button(ButtonLed::LiveOverwrite)),
            Button::Cam4 => Some(Led::Button(ButtonLed::Cam4)),
            Button::Cam5 => Some(Led::Button(ButtonLed::Cam5)),
            Button::Cam6 => Some(Led::Button(ButtonLed::Cam6)),
            Button::VideoOnly => Some(Led::Button(ButtonLed::VideoOnly)),
            Button::Cam1 => Some(Led::Button(ButtonLed::Cam1)),
            Button::Cam2 => Some(Led::Button(ButtonLed::Cam2)),
            Button::Cam3 => Some(Led::Button(ButtonLed::Cam3)),
            Button::AudioOnly => Some(Led::Button(ButtonLed::AudioOnly)),

            Button::Shuttle => Some(Led::Wheel(WheelLed::Shuttle)),
            Button::Jog => Some(Led::Wheel(WheelLed::Jog)),
            Button::Scroll => Some(Led::Wheel(WheelLed::Scroll)),

            _ => None,
        }
    }
}

impl TryFrom<u16> for Button {
    type Error = crate::Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0x0001 => Ok(Button::SmartInsert),
            0x0002 => Ok(Button::Append),
            0x0003 => Ok(Button::RippleOverwrite),
            0x0004 => Ok(Button::CloseUp),
            0x0005 => Ok(Button::PlaceOnTop),
            0x0006 => Ok(Button::SourceOverwrite),
            0x0007 => Ok(Button::In),
            0x0008 => Ok(Button::Out),
            0x0009 => Ok(Button::TrimIn),
            0x000a => Ok(Button::TrimOut),
            0x000b => Ok(Button::Roll),
            0x000c => Ok(Button::SlipSource),
            0x000d => Ok(Button::SlipDestination),
            0x000e => Ok(Button::TransitionDuration),
            0x000f => Ok(Button::Cut),
            0x0010 => Ok(Button::Dissolve),
            0x0011 => Ok(Button::SmoothCut),
            0x0031 => Ok(Button::Escape),
            0x001f => Ok(Button::SyncBin),
            0x002c => Ok(Button::AudioLevel),
            0x002d => Ok(Button::FullView),
            0x0022 => Ok(Button::Transition),
            0x002f => Ok(Button::Split),
            0x002e => Ok(Button::Snap),
            0x002b => Ok(Button::RippleDelete),
            0x0033 => Ok(Button::Cam1),
            0x0034 => Ok(Button::Cam2),
            0x0035 => Ok(Button::Cam3),
            0x0036 => Ok(Button::Cam4),
            0x0037 => Ok(Button::Cam5),
            0x0038 => Ok(Button::Cam6),
            0x0039 => Ok(Button::Cam7),
            0x003a => Ok(Button::Cam8),
            0x003b => Ok(Button::Cam9),
            0x0030 => Ok(Button::LiveOverwrite),
            0x0025 => Ok(Button::VideoOnly),
            0x0026 => Ok(Button::AudioOnly),
            0x003c => Ok(Button::StopPlay),
            0x001a => Ok(Button::Source),
            0x001b => Ok(Button::Timeline),
            0x001c => Ok(Button::Shuttle),
            0x001d => Ok(Button::Jog),
            0x001e => Ok(Button::Scroll),
            _ => Err(crate::Error::Driver { message: "invalid button value received" }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WheelMode {
    Relative = 0x00,
    AbsoluteContinuous = 0x01,
    AbsoluteDeadZero = 0x03,
}

impl TryFrom<u8> for WheelMode {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(WheelMode::Relative),
            0x01 => Ok(WheelMode::AbsoluteContinuous),
            0x02 => Ok(WheelMode::Relative), // NOTE: 0x00 and 0x02 appear to be the same.
            0x03 => Ok(WheelMode::AbsoluteDeadZero),
            _ => Err(crate::Error::Driver { message: "received invalid wheel mode" }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ButtonLed {
    #[default]
    Off = 0,

    CloseUp = 1 << 0,
    Cut = 1 << 1,
    Dissolve = 1 << 2,
    SmoothCut = 1 << 3,
    Transition = 1 << 4,
    Snap = 1 << 5,
    Cam7 = 1 << 6,
    Cam8 = 1 << 7,
    Cam9 = 1 << 8,
    LiveOverwrite = 1 << 9,
    Cam4 = 1 << 10,
    Cam5 = 1 << 11,
    Cam6 = 1 << 12,
    VideoOnly = 1 << 13,
    Cam1 = 1 << 14,
    Cam2 = 1 << 15,
    Cam3 = 1 << 16,
    AudioOnly = 1 << 17,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum WheelLed {
    #[default]
    Off = 0,

    Jog = 1 << 0,
    Shuttle = 1 << 1,
    Scroll = 1 << 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Led {
    Button(ButtonLed),
    Wheel(WheelLed),
}

pub fn get_hid_device() -> Result<HidDevice, crate::Error> {
    let api = HidApi::new().map_err(|_| crate::Error::HidApiAlreadyInitialized)?;

    let device = api.open(VENDOR_ID, PRODUCT_ID).map_err(|_| crate::Error::CannotOpenHidDevice)?;

    Ok(device)
}

pub fn authenticate(device: &mut HidDevice) -> Result<u16, crate::Error> {
    let mut buf = [0x00; 10];

    // The authentication is performed over SET_FEATURE/GET_FEATURE on
    // Report ID 6

    // Reset the auth state machine
    device
        .send_feature_report(&[0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
        .map_err(|_| crate::Error::Driver { message: "failed to send auth reset" })?;

    fn get_feature<'a>(
        buf: &'a mut [u8; 10],
        device: &HidDevice,
    ) -> Result<&'a [u8], crate::Error> {
        // Prepare buffer and set the Report ID (0x06) before requesting it.
        // hidapi requires buf[0] to contain the report id for GET_FEATURE.
        *buf = [0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let len = device
            .get_feature_report(buf)
            .map_err(|_| crate::Error::Driver { message: "failed to get feature report" })?;
        Ok(&buf[..len])
    }

    // Read the keyboard challenge (for keyboard to authenticate app)
    let data = get_feature(&mut buf, device)
        .map_err(|_| crate::Error::Driver { message: "failed to get keyboard challenge" })?;
    if data.len() < 10 {
        return Err(crate::Error::Driver { message: "authentication failed" });
    }
    if data[0] != 0x06 || data[1] != 0x00 {
        return Err(crate::Error::Driver { message: "authentication failed" });
    }
    let challenge = u64::from_le_bytes(data[2..10].try_into().map_err(|_| {
        crate::Error::Driver { message: "failed to parse keyboard challenge bytes" }
    })?);

    // Send our challenge (to authenticate keyboard)
    // We don't care ... so just send 0x0000000000000000
    device
        .send_feature_report(&[0x06, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
        .map_err(|_| crate::Error::Driver { message: "failed to send challenge feature" })?;

    // Read the keyboard response
    // Again, we don't care, ignore the result
    let data = get_feature(&mut buf, device)
        .map_err(|_| crate::Error::Driver { message: "failed to get keyboard response" })?;
    if data.len() < 10 {
        return Err(crate::Error::Driver { message: "authentication failed" });
    }
    if data[0] != 0x06 || data[1] != 0x02 {
        return Err(crate::Error::Driver { message: "authentication failed" });
    }

    // Compute and send our response
    let response = bmd_kbd_auth(challenge);
    let rb = response.to_le_bytes();
    device
        .send_feature_report(&[0x06, 0x03, rb[0], rb[1], rb[2], rb[3], rb[4], rb[5], rb[6], rb[7]])
        .map_err(|_| crate::Error::Driver { message: "failed to send challenge response" })?;

    // Read the status
    let data = get_feature(&mut buf, device)
        .map_err(|_| crate::Error::Driver { message: "failed to get status" })?;
    if data.len() < 10 {
        return Err(crate::Error::Driver { message: "authentication failed" });
    }
    if data[0] != 0x06 || data[1] != 0x04 {
        return Err(crate::Error::Driver { message: "authentication failed" });
    }

    // I "think" what gets returned here is the timeout after which auth
    // needs to be done again (returns 600 for me which is plausible)
    Ok(u16::from_le_bytes([data[2], data[3]]))
}

pub fn set_button_led(device: &mut HidDevice, led: ButtonLed) -> Result<(), crate::Error> {
    let mut buf = [0u8; 5];
    buf[0] = 2;
    buf[1..5].copy_from_slice(&(led as u32).to_le_bytes());
    device
        .write(&buf)
        .map_err(|_| crate::Error::Driver { message: "failed to write LED state" })?;
    Ok(())
}

pub fn set_wheel_led(device: &mut HidDevice, led: WheelLed) -> Result<(), crate::Error> {
    let buf = [4u8, led as u8];
    device
        .write(&buf)
        .map_err(|_| crate::Error::Driver { message: "failed to write wheel LED state" })?;
    Ok(())
}

pub fn _set_wheel_mode(device: &mut HidDevice, wheel_mode: WheelMode) {
    let mut buf = [0u8; 7];
    buf[0] = 3;
    buf[1] = wheel_mode as u8;
    buf[2..6].copy_from_slice(&0u32.to_le_bytes());
    buf[6] = 0; // unknown
    let _ = device.write(&buf);
}

fn bmd_kbd_auth(challenge: u64) -> u64 {
    const AUTH_EVEN_TBL: [u64; 8] = [
        0x3ae1206f97c10bc8,
        0x2a9ab32bebf244c6,
        0x20a6f8b8df9adf0a,
        0xaf80ece52cfc1719,
        0xec2ee2f7414fd151,
        0xb055adfd73344a15,
        0xa63d2e3059001187,
        0x751bf623f42e0dde,
    ];

    const AUTH_ODD_TBL: [u64; 8] = [
        0x3e22b34f502e7fde,
        0x24656b981875ab1c,
        0xa17f3456df7bf8c3,
        0x6df72e1941aef698,
        0x72226f011e66ab94,
        0x3831a3c606296b42,
        0xfd7ff81881332c89,
        0x61a3f6474ff236c6,
    ];

    const MASK: u64 = 0xa79a63f585d37bf0;

    let ror8n = |mut v: u64, n: usize| -> u64 {
        for _ in 0..n {
            v = v.rotate_right(8);
        }
        v
    };

    let n = (challenge & 7) as usize;
    let mut v = ror8n(challenge, n);

    let k = if (v & 1) == ((0x78 >> n) & 1) {
        AUTH_EVEN_TBL[n]
    } else {
        v = v ^ v.rotate_right(8);
        AUTH_ODD_TBL[n]
    };

    v ^ (v.rotate_right(8) & MASK) ^ k
}

pub fn poll(device: &mut HidDevice, timeout: i32) -> Result<Report, crate::Error> {
    let mut buf = [0x00; 64];
    let len = device
        .read_timeout(&mut buf, timeout)
        .map_err(|_| crate::Error::Driver { message: "failed to read" })?;
    if len == 0 {
        return Err(crate::Error::Driver { message: "received empty report" });
    }
    let report_bytes = &buf[0..len];

    let report = Report::try_from(report_bytes)
        .map_err(|_| crate::Error::Driver { message: "failed to parse report" })?;

    Ok(report)
}
