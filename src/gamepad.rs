use gilrs::{Axis, Button, Gamepad};
use macroquad::prelude::*;
use retro_rs::{Buttons, InputPort};

pub fn update_input_port_with_gamepad(input: &mut InputPort, g: &Gamepad) {
    input.buttons = Buttons::new()
        .up(g.is_pressed(Button::DPadUp))
        .down(g.is_pressed(Button::DPadDown))
        .left(g.is_pressed(Button::DPadLeft))
        .right(g.is_pressed(Button::DPadRight))
        .a(g.is_pressed(Button::East))
        .b(g.is_pressed(Button::South))
        .x(g.is_pressed(Button::North))
        .y(g.is_pressed(Button::West))
        .l1(g.is_pressed(Button::LeftTrigger))
        .r1(g.is_pressed(Button::RightTrigger))
        .l2(g.is_pressed(Button::LeftTrigger2))
        .r2(g.is_pressed(Button::RightTrigger2))
        .l3(g.is_pressed(Button::LeftThumb))
        .r3(g.is_pressed(Button::RightThumb))
        .start(g.is_pressed(Button::Start))
        .select(g.is_pressed(Button::Select));

    let (x, y) = get_stick(g);
    input.joystick_x = (x * 32766.0) as i16;
    input.joystick_y = (-y * 32766.0) as i16;
}

pub fn update_input_port_with_keyboard(input: &mut InputPort) {
    input.buttons = Buttons::new()
        .up(is_key_down(KeyCode::Up))
        .down(is_key_down(KeyCode::Down))
        .left(is_key_down(KeyCode::Left))
        .right(is_key_down(KeyCode::Right))
        .a(is_key_down(KeyCode::D))
        .b(is_key_down(KeyCode::S))
        .x(is_key_down(KeyCode::W))
        .y(is_key_down(KeyCode::A))
        .l1(is_key_down(KeyCode::Q))
        .r1(is_key_down(KeyCode::E))
        .l2(is_key_down(KeyCode::Z))
        .r2(is_key_down(KeyCode::C))
        .start(is_key_down(KeyCode::Enter))
        .select(is_key_down(KeyCode::Backspace));

    {
        input.mouse_left_down = is_mouse_button_down(MouseButton::Left);
        input.mouse_right_down = is_mouse_button_down(MouseButton::Right);
        input.mouse_middle_down = is_mouse_button_down(MouseButton::Middle);

        input.joystick_x = if is_key_down(KeyCode::J) {
            -50
        } else if is_key_down(KeyCode::L) {
            50
        } else {
            0
        };

        input.joystick_y = if is_key_down(KeyCode::I) {
            50
        } else if is_key_down(KeyCode::K) {
            -50
        } else {
            0
        };
    }
}

pub fn get_stick(gamepad: &Gamepad) -> (f32, f32) {
    let x = gamepad.axis_data(Axis::LeftStickX);
    let y = gamepad.axis_data(Axis::LeftStickY);
    x.zip(y)
        .map(|(x, y)| (x.value(), y.value()))
        .unwrap_or((0.0, 0.0))
}
