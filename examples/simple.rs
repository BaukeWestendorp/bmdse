use bmdse::{ButtonLed, SpeedEditor, WheelLed};

fn main() {
    let mut speed_editor = SpeedEditor::new()
        .unwrap()
        .on_wheel_change(|velocity| {
            eprintln!("wheel velocity: {velocity}");
        })
        .on_button_change(|button, pressed| {
            eprintln!("button {button:?} {}", if pressed { "pressed" } else { "released" });
        })
        .on_battery_info(|charging, battery| {
            eprintln!(
                "charging: {charging} | battery percentage: {:.0}%",
                (battery as f32 / u8::MAX as f32) * 100.0
            );
        });

    speed_editor.set_button_led(ButtonLed::Cam1).unwrap();
    speed_editor.set_wheel_led(WheelLed::Jog).unwrap();

    // Because the SpeedEditor spawns a new thread handling input,
    // we have to keep the main thread running.
    loop {}
}
