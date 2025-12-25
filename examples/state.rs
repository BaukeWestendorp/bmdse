use std::{
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use bmdse::{Button, SpeedEditor};

fn main() {
    // Because we go over a thread boundary inside the event handler callbacks,
    // we have to wrap the state in Arc<RwLock<T>>.
    let state = Arc::new(RwLock::new(State::default()));

    let _speed_editor = SpeedEditor::new()
        .unwrap()
        .on_wheel_change({
            let state = Arc::clone(&state);
            move |velocity| {
                let mut state_guard = state.write().unwrap();
                state_guard.absolute_wheel_value += velocity as i64;
            }
        })
        .on_button_change({
            let state = Arc::clone(&state);
            move |button, pressed| {
                if !pressed {
                    return;
                };

                let mode = match button {
                    Button::Timeline => Mode::Timeline,
                    Button::Source => Mode::Source,
                    _ => return,
                };

                let mut state_guard = state.write().unwrap();
                state_guard.mode = mode;
            }
        });

    loop {
        eprintln!("{:?}", state.read().unwrap());
        thread::sleep(Duration::from_millis(20));
    }
}

#[derive(Debug, Default)]
struct State {
    mode: Mode,
    absolute_wheel_value: i64,
}

#[derive(Debug, Default)]
enum Mode {
    #[default]
    Source,
    Timeline,
}
