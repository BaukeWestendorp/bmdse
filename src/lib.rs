use std::{
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

mod driver;
mod error;

use hidapi::HidDevice;

use crate::driver::WheelMode;

pub use crate::driver::{Button, ButtonLed, Report, WheelLed};
pub use crate::error::Error;

pub struct SpeedEditor {
    inner: Arc<Mutex<Inner>>,
}

impl SpeedEditor {
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

    pub fn on_wheel_change<F: Fn(i32) + Send + 'static>(mut self, f: F) -> Self {
        self.set_on_wheel_change(f);
        self
    }

    pub fn set_on_wheel_change<F: Fn(i32) + Send + 'static>(&mut self, f: F) {
        self.inner.lock().unwrap().on_wheel_change = Some(Box::new(f));
    }

    pub fn on_button_change<F: Fn(Button, bool) + Send + 'static>(mut self, f: F) -> Self {
        self.set_on_button_change(f);
        self
    }

    pub fn set_on_button_change<F: Fn(Button, bool) + Send + 'static>(&mut self, f: F) {
        self.inner.lock().unwrap().on_button_change = Some(Box::new(f));
    }

    pub fn on_battery_info<F: Fn(bool, u8) + Send + 'static>(mut self, f: F) -> Self {
        self.set_on_battery_info(f);
        self
    }

    pub fn set_on_battery_info<F: Fn(bool, u8) + Send + 'static>(&mut self, f: F) {
        self.inner.lock().unwrap().on_battery_info = Some(Box::new(f));
    }

    pub fn is_button_pressed(&self, button: Button) -> bool {
        self.inner.lock().unwrap().pressed_buttons.contains(&button)
    }

    pub fn pressed_buttons(&self) -> Vec<Button> {
        self.inner.lock().unwrap().pressed_buttons.to_owned()
    }

    pub fn set_wheel_led(&mut self, led: WheelLed) -> Result<(), crate::Error> {
        self.inner.lock().unwrap().wheel_led = led;
        Ok(())
    }

    pub fn get_wheel_led(&self) -> WheelLed {
        self.inner.lock().unwrap().wheel_led
    }

    pub fn set_button_led(&mut self, led: ButtonLed) -> Result<(), crate::Error> {
        self.inner.lock().unwrap().button_led = led;
        Ok(())
    }

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
