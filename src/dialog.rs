use macroquad::prelude::*;

use crate::AppEvent;

pub enum DynamicDialog {
    YesOrNo(YesOrNoDialog),
    //Login(LoginDialog),
    //Message(MessageDialog),
    //Options(Vec<String>),
}

pub trait Dialog {
    type Value;

    fn update(&mut self) -> DialogUpdate;
    fn render(&self);
    fn current_value(&self) -> Self::Value;
    fn produce_event(self) -> AppEvent;
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DialogUpdate {
    Finish,
    Continue,
}

pub struct YesOrNoDialog {
    pub text: String,
    pub value: bool,
    pub event_handler: Box<dyn FnOnce(bool) -> AppEvent>,
}

impl Dialog for YesOrNoDialog {
    type Value = bool;

    fn update(&mut self) -> DialogUpdate {
        let change = is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::Right);

        if change {
            self.value = !self.value;
        }

        if is_key_pressed(KeyCode::Enter) {
            DialogUpdate::Finish
        } else {
            DialogUpdate::Continue
        }
    }

    fn render(&self) {
        let (sw, sh) = (screen_width(), screen_height());
        let width = sw / 1.2;
        let height = sh / 1.2;
        let x = (sw / 2.0) - (width / 2.0);
        let y = (sh / 2.0) - (height / 2.0);

        let margin = 2.0;
        let white = Color::from_rgba(255, 255, 255, 255);
        let yellow = Color::from_rgba(255, 255, 0, 255);

        draw_rectangle(x, y, width, height, Color::from_rgba(0, 0, 0, 255));
        draw_text(
            &self.text,
            x + margin,
            y + margin + 64.0,
            32.0,
            Color::from_rgba(255, 255, 255, 255),
        );
        draw_text(
            "Yes",
            x + margin,
            y + margin + 128.0,
            32.0,
            if self.value { yellow } else { white },
        );
        draw_text(
            "No",
            x + margin + (width / 2.0),
            y + margin + 128.0,
            32.0,
            if !self.value { yellow } else { white },
        );
    }

    fn current_value(&self) -> Self::Value {
        self.value
    }

    fn produce_event(self) -> AppEvent {
        (self.event_handler)(self.value)
    }
}
