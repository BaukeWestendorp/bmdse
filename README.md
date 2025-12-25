# Black Magic Design Speed Editor

An interface for talking with a Black Magic Design Speed Editor using the HID API in written in Rust.

![Black Magic Design Speed Editor next to my cat Julius](bmdse.jpg)

This library was created because I was missing MIDI functionality for the Speed Editor, when experimenting with controllers for my other project [zeevonk](https://github.com/BaukeWestendorp/zeevonk). I ended up writing this high-level API, with an internal low(er)-level driver, as I did not want to constantly manage another thread for the event polling in each of my small testing-purpouse applications.

Thanks to [Sylvain "tnt" Munaut](https://github.com/smunaut/blackmagic-misc) for reverse
engineering the difficult parts like authentication!

## Example

```rust
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
```

You can run the examples using

`cargo run --release --example simple` or `cargo run --release --example state`

## Known Problems

Sometimes the Speed Editor HID device is opened, does not receive events when connected using bluetooth.
If anyone knows what is going on here, feel free to create an issue, or even better a PR!

## Contributing

Feel free to start an issue to discuss about missing features or found bugs, it's OSS after all!
