use std::{error, fmt, io};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),

    Driver { message: &'static str },

    HidDeviceNotFound,
    HidApiAlreadyInitialized,
    CannotOpenHidDevice,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::Driver { message } => write!(f, "Driver error: {}", message),
            Error::HidDeviceNotFound => write!(f, "HID device not found"),
            Error::HidApiAlreadyInitialized => write!(f, "HID API already initialized"),
            Error::CannotOpenHidDevice => write!(f, "cannot open HID device"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}
