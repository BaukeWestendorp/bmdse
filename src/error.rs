use std::{error, fmt, io};

/// Various error variants used in `bmdse`.
#[derive(Debug)]
pub enum Error {
    /// An [io::Error][std::io::Error].
    Io(io::Error),

    /// An error that occured in the driver.
    Driver {
        /// Information about what went wrong.
        message: &'static str,
    },

    /// The BMD Speed Editor HID device was not found.
    HidDeviceNotFound,
    /// The HID API already has been initialized.
    HidApiAlreadyInitialized,
    /// Could not open the BMD Speed Editor HID device.
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
