#![warn(missing_docs)]

//! # Black Magic Design Speed Editor
//!
//! An interface for talking with a Black Magic Design Speed Editor using the HID API in written in Rust.
//!
//! ## Example
//!
//! ```rust
//! use std::{
//!     sync::{Arc, RwLock},
//!     thread,
//!     time::Duration,
//! };
//!
//! use bmdse::{Button, SpeedEditor};
//!
//! fn main() {
//!     // Because we go over a thread boundary inside the event handler callbacks,
//!     // we have to wrap the state in Arc<RwLock<T>>.
//!     let state = Arc::new(RwLock::new(State::default()));
//!
//!     let _speed_editor = SpeedEditor::new()
//!         .unwrap()
//!         .on_wheel_change({
//!             let state = Arc::clone(&state);
//!             move |velocity| {
//!                 let mut state_guard = state.write().unwrap();
//!                 state_guard.absolute_wheel_value += velocity as i64;
//!             }
//!         })
//!         .on_button_change({
//!             let state = Arc::clone(&state);
//!             move |button, pressed| {
//!                 if !pressed {
//!                     return;
//!                 };
//!
//!                 let mode = match button {
//!                     Button::Timeline => Mode::Timeline,
//!                     Button::Source => Mode::Source,
//!                     _ => return,
//!                 };
//!
//!                 let mut state_guard = state.write().unwrap();
//!                 state_guard.mode = mode;
//!             }
//!         });
//!
//!     loop {
//!         eprintln!("{:?}", state.read().unwrap());
//!         thread::sleep(Duration::from_millis(20));
//!     }
//! }
//!
//! #[derive(Debug, Default)]
//! struct State {
//!     mode: Mode,
//!     absolute_wheel_value: i64,
//! }
//!
//! #[derive(Debug, Default)]
//! enum Mode {
//!     #[default]
//!     Source,
//!     Timeline,
//! }
//! ```
//!
//! You can run the examples using
//!
//! `cargo run --release --example simple` or `cargo run --release --example state`

use std::{
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

mod driver;
mod error;

use hidapi::HidDevice;

use crate::driver::{Report, WheelMode};

pub use crate::driver::{Button, ButtonLed, WheelLed};
pub use crate::error::Error;

/// The main interface to talk with the Speed Editor device.
///
/// It has a few callbacks you can use to listen for events (e.g. wheel changes, button presses or battery information),
/// and some functions you can use to directly ask about the state of the device (e.g. button state, LED state or battery information).
///
/// On creation, it will spawn a new thread handling all the event polling, so you do not need to think about that.
///
/// # Example
///
/// ```
/// use bmdse::{ButtonLed, SpeedEditor, WheelLed};
///
/// fn main() {
///     let mut speed_editor = SpeedEditor::new()
///         .unwrap()
///         .on_wheel_change(|velocity| {
///             eprintln!("wheel velocity: {velocity}");
///         })
///         .on_button_change(|button, pressed| {
///             eprintln!("button {button:?} {}", if pressed { "pressed" } else { "released" });
///         })
///         .on_battery_info(|charging, percentage| {
///             eprintln!("charging: {charging} | battery percentage: {percentage}%");
///         });
///
///     speed_editor.set_button_led(ButtonLed::Cam1);
///     speed_editor.set_wheel_led(WheelLed::Jog);
///
///     // Because the SpeedEditor spawns a new thread handling input,
///     // we have to keep the main thread running.
///     loop {}
/// }
/// ```
pub struct SpeedEditor {
    inner: Arc<Mutex<Inner>>,
}

impl SpeedEditor {
    /// Creates a new [`SpeedEditor`].
    ///
    /// # Errors
    ///
    /// This function might error when getting the HID device
    /// (cannot be found, HID API already initialized, etc.).
    ///
    /// It will spawn a new thread, that handles all event polling.
    pub fn new() -> Result<Self, crate::Error> {
        let inner = Arc::new(Mutex::new(Inner {
            pressed_buttons: Vec::new(),

            button_led: ButtonLed::default(),
            wheel_led: WheelLed::default(),

            on_wheel_change: None,
            on_button_change: None,
            on_battery_info: None,
        }));

        let hid_device = driver::get_hid_device()?;
        thread::Builder::new().name("bmd_speed_editor_poller".to_string()).spawn({
            let inner = Arc::clone(&inner);
            move || poller(hid_device, inner)
        })?;

        Ok(Self { inner })
    }

    /// Provide a callback to handle a change of the jog wheel,
    /// with its parameter being the wheel's velocity.
    pub fn on_wheel_change<F: Fn(i32) + Send + 'static>(mut self, f: F) -> Self {
        self.set_on_wheel_change(f);
        self
    }

    /// Provide a callback to handle a change of the jog wheel,
    /// with it's parameter being the wheel's velocity.
    pub fn set_on_wheel_change<F: Fn(i32) + Send + 'static>(&mut self, f: F) {
        self.inner.lock().unwrap().on_wheel_change = Some(Box::new(f));
    }

    /// Provide a callback to handle a press or release of a button,
    /// with its first parameter being the button,
    /// and its second parameter telling if it's pressed (`true`) or released (`false`).
    pub fn on_button_change<F: Fn(Button, bool) + Send + 'static>(mut self, f: F) -> Self {
        self.set_on_button_change(f);
        self
    }

    /// Provide a callback to handle a press or release of a button,
    /// with its first parameter being the button,
    /// and its second parameter telling if it's pressed (`true`) or released (`false`).
    pub fn set_on_button_change<F: Fn(Button, bool) + Send + 'static>(&mut self, f: F) {
        self.inner.lock().unwrap().on_button_change = Some(Box::new(f));
    }

    /// Provide a callback to handle battery info,
    /// with it's first parameter telling if it's charging, and the second parameter being
    /// the battery percentage (`0..=100`).
    pub fn on_battery_info<F: Fn(bool, u8) + Send + 'static>(mut self, f: F) -> Self {
        self.set_on_battery_info(f);
        self
    }

    /// Provide a callback to handle battery info,
    /// with it's first parameter telling if it's charging, and the second parameter being
    /// the battery percentage (`0..=100`).
    pub fn set_on_battery_info<F: Fn(bool, u8) + Send + 'static>(&mut self, f: F) {
        self.inner.lock().unwrap().on_battery_info = Some(Box::new(f));
    }

    /// Returns `true` if the provided button is currently pressed.
    pub fn is_button_pressed(&self, button: Button) -> bool {
        self.inner.lock().unwrap().pressed_buttons.contains(&button)
    }

    /// Returns a all currently pressed buttons.
    pub fn pressed_buttons(&self) -> Vec<Button> {
        self.inner.lock().unwrap().pressed_buttons.to_owned()
    }

    /// Set the current wheel LED state.
    pub fn set_wheel_led(&mut self, led: WheelLed) {
        self.inner.lock().unwrap().wheel_led = led;
    }

    /// Get the current wheel LED state.
    pub fn get_wheel_led(&self) -> WheelLed {
        self.inner.lock().unwrap().wheel_led
    }

    /// Set the current button LED state.
    pub fn set_button_led(&mut self, led: ButtonLed) {
        self.inner.lock().unwrap().button_led = led;
    }

    /// Get the current button LED state.
    pub fn get_button_led(&self) -> ButtonLed {
        self.inner.lock().unwrap().button_led
    }
}

fn poller(mut hid_device: HidDevice, inner: Arc<Mutex<Inner>>) -> Result<(), crate::Error> {
    const MAX_POLL_MS: i32 = 16;

    let mut auth_time = 600;
    let auth_instant = Instant::now();

    let mut last_button_led = None;
    let mut last_wheel_led = None;

    driver::authenticate(&mut hid_device)?;

    loop {
        if auth_instant.elapsed().as_secs() >= (auth_time - 5) as u64 {
            auth_time = driver::authenticate(&mut hid_device)?;
        }

        {
            let inner_guard = inner.lock().unwrap();
            if last_button_led.is_none_or(|last_led| last_led != inner_guard.button_led) {
                driver::set_button_led(&mut hid_device, inner_guard.button_led)?;
                last_button_led = Some(inner_guard.button_led);
            }
            if last_wheel_led.is_none_or(|last_led| last_led != inner_guard.wheel_led) {
                driver::set_wheel_led(&mut hid_device, inner_guard.wheel_led)?;
                last_wheel_led = Some(inner_guard.wheel_led);
            }
        }

        let report = match driver::poll(&mut hid_device, MAX_POLL_MS) {
            Ok(report) => report,
            Err(_) => {
                std::thread::yield_now();
                continue;
            }
        };

        match report {
            Report::Wheel { mode, value } => {
                if let WheelMode::Relative = mode {
                    let inner_guard = inner.lock().unwrap();

                    if let Some(on_wheel_change) = &inner_guard.on_wheel_change {
                        on_wheel_change(value);
                    }
                }
            }
            Report::Buttons(buttons) => {
                let mut inner_guard = inner.lock().unwrap();

                // Save previous pressed buttons for comparison
                let prev_pressed = inner_guard.pressed_buttons.clone();
                inner_guard.pressed_buttons = buttons.clone();

                if let Some(on_button_change) = &inner_guard.on_button_change {
                    // For all buttons that were previously pressed but are not in the new list, call with false
                    for button in prev_pressed.iter() {
                        if !buttons.contains(button) {
                            on_button_change(*button, false);
                        }
                    }

                    // For all buttons that are currently pressed, call with true
                    for button in &buttons {
                        on_button_change(*button, true);
                    }
                }
            }
            Report::Battery { charging, level } => {
                let inner_guard = inner.lock().unwrap();

                if let Some(on_battery_info) = &inner_guard.on_battery_info {
                    on_battery_info(charging, level);
                }
            }
        }
    }
}

struct Inner {
    pressed_buttons: Vec<Button>,
    button_led: ButtonLed,
    wheel_led: WheelLed,

    on_wheel_change: Option<Box<dyn Fn(i32) + Send>>,
    on_button_change: Option<Box<dyn Fn(Button, bool) + Send>>,
    on_battery_info: Option<Box<dyn Fn(bool, u8) + Send>>,
}
