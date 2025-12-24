use std::{
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use crate::driver::{Button, ButtonLed, Report, WheelLed, WheelMode};

mod driver;
mod error;

pub use error::Error;

pub struct SpeedEditor {
    inner: Arc<Mutex<Inner>>,
}

impl SpeedEditor {
    pub fn new() -> Result<Self, crate::Error> {
        let inner = Arc::new(Mutex::new(Inner {
            absolute_wheel_value: Default::default(),
            pressed_buttons: Vec::new(),

            button_led: ButtonLed::default(),
            wheel_led: WheelLed::default(),

            on_wheel_change: None,
            on_button_change: None,
        }));

        thread::Builder::new().name("bmd_speed_editor_poller".to_string()).spawn({
            let inner = Arc::clone(&inner);
            move || poller(inner)
        })?;

        Ok(Self { inner })
    }

    pub fn on_wheel_change<F: Fn(i32, i64) + Send + 'static>(mut self, f: F) -> Self {
        self.set_on_wheel_change(f);
        self
    }

    pub fn set_on_wheel_change<F: Fn(i32, i64) + Send + 'static>(&mut self, f: F) {
        self.inner.lock().unwrap().on_wheel_change = Some(Box::new(f));
    }

    pub fn on_button_change<F: Fn(Button, bool) + Send + 'static>(mut self, f: F) -> Self {
        self.set_on_button_change(f);
        self
    }

    pub fn set_on_button_change<F: Fn(Button, bool) + Send + 'static>(&mut self, f: F) {
        self.inner.lock().unwrap().on_button_change = Some(Box::new(f));
    }

    pub fn absolute_wheel_value(&mut self) -> i64 {
        self.inner.lock().unwrap().absolute_wheel_value
    }

    pub fn reset_absolute_wheel_value(&mut self) {
        self.inner.lock().unwrap().absolute_wheel_value = Default::default();
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

fn poller(inner: Arc<Mutex<Inner>>) -> Result<(), crate::Error> {
    const MAX_POLL_MS: i32 = 16;

    let mut hid_device = driver::get_hid_device()?;

    let mut auth_time = driver::authenticate(&mut hid_device)?;
    let auth_instant = Instant::now();

    let mut last_button_led = None;
    let mut last_wheel_led = None;

    loop {
        // FIXME: The authentication does not repeat for some reason.
        if auth_instant.elapsed().as_secs() >= (auth_time - 5) as u64 {
            auth_time = driver::authenticate(&mut hid_device)?;
        }

        {
            // FIXME: There must be a nicer way of doing these matches.
            let inner_guard = inner.lock().unwrap();
            match (last_button_led, inner_guard.button_led) {
                (Some(last_led), led) if last_led != led => {
                    driver::set_button_led(&mut hid_device, led)?;
                    last_button_led = Some(led);
                }
                (None, led) => {
                    driver::set_button_led(&mut hid_device, led)?;
                    last_button_led = Some(led);
                }
                _ => {}
            }
            match (last_wheel_led, inner_guard.wheel_led) {
                (Some(last_led), led) if last_led != led => {
                    driver::set_wheel_led(&mut hid_device, led)?;
                    last_wheel_led = Some(led);
                }
                (None, led) => {
                    driver::set_wheel_led(&mut hid_device, led)?;
                    last_wheel_led = Some(led);
                }
                _ => {}
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
                    let mut inner = inner.lock().unwrap();

                    inner.absolute_wheel_value += value as i64;

                    if let Some(on_wheel_change) = &inner.on_wheel_change {
                        on_wheel_change(value, inner.absolute_wheel_value);
                    }
                }
            }
            Report::Buttons(buttons) => {
                let mut inner = inner.lock().unwrap();

                // Save previous pressed buttons for comparison
                let prev_pressed = inner.pressed_buttons.clone();
                inner.pressed_buttons = buttons.clone();

                if let Some(on_button_change) = &inner.on_button_change {
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
            Report::Battery { .. } => {
                // FIXME: Do something with battery information.
            }
        }
    }
}

struct Inner {
    absolute_wheel_value: i64,
    pressed_buttons: Vec<Button>,
    button_led: ButtonLed,
    wheel_led: WheelLed,

    on_wheel_change: Option<Box<dyn Fn(i32, i64) + Send>>,
    on_button_change: Option<Box<dyn Fn(Button, bool) + Send>>,
}
