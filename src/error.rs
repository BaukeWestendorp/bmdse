#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Driver error: {message}")]
    Driver { message: &'static str },

    #[error("HID device not found")]
    HidDeviceNotFound,
    #[error("HID API already initialized")]
    HidApiAlreadyInitialized,
    #[error("cannot open HID device")]
    CannotOpenHidDevice,
}
