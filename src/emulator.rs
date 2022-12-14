use std::{
    collections::HashSet,
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use cpal::traits::DeviceTrait;
use gilrs::{Button, Event, GamepadId, Gilrs};
use libretro_sys::PixelFormat;
use macroquad::prelude::*;
use retro_rs::{pixels, Emulator, InputPort, RetroRsError};

use crate::{
    audio,
    gamepad::{update_input_port_with_gamepad, update_input_port_with_keyboard},
    AppEvent,
};

pub struct EmulatorState {
    emu: Emulator,
    controllers: [InputPort; 2],
    gamepad_ids: HashSet<GamepadId>,

    // Graphics
    fb_copy: Vec<u8>,
    fb_image: Image,
    fb_texture: Texture2D,
    fb_interlace_factor: usize,

    // Audio
    #[allow(dead_code)]
    audio_device: cpal::Device,
    #[allow(dead_code)]
    audio_stream: cpal::Stream,
    audio_buffer: Arc<Mutex<Vec<i16>>>,
}

impl EmulatorState {
    pub fn create(core: &Path, rom: &Path, save: Option<Vec<u8>>) -> Self {
        let mut emu = Emulator::create(core, rom);
        let controllers = [InputPort::new(), InputPort::new()];

        emu.run(controllers);
        emu.reset();

        // Load save state if given
        if let Some(save) = save {
            emu.run(controllers);
            emu.run(controllers);
            emu.run(controllers);

            println!("INFO: Loading provided save file state");
            emu.load(&save);
        }

        let (width, height) = emu.framebuffer_size();
        let pitch = emu.framebuffer_pitch();

        let fb_copy = vec![0u8; height * pitch];

        let fb_image = Image {
            bytes: [0x00, 0x00, 0x00, 0xFF].repeat(width * height),
            width: width as u16,
            height: height as u16,
        };

        let fb_texture = Texture2D::from_image(&fb_image);
        fb_texture.set_filter(FilterMode::Nearest);
        let fb_interlace_factor = 1;

        let audio_device = audio::init().unwrap();
        let audio_buffer = Arc::new(Mutex::new(Vec::new()));

        let audio_stream = audio::run(&audio_device, {
            let audio_buffer = audio_buffer.clone();

            // Get device sample rate
            let default_output_config = audio_device.default_output_config().unwrap();
            let device_sample_rate = default_output_config.sample_rate().0 as f64;

            // Get core sample rate
            let av_info = emu.system_av_info();
            let core_sample_rate = av_info.timing.sample_rate;

            let resample_rate = core_sample_rate / device_sample_rate;
            println!(
                "AUDIO: Device sample rate {}; Core sample rate: {} Resample rate {}",
                device_sample_rate, core_sample_rate, resample_rate
            );
            println!(
                "AUDIO: Device buffer size {:?}",
                default_output_config.buffer_size()
            );
            //let mut audio_buffer_resampled = Vec::new();

            move |output_buf| {
                let mut core_buf = audio_buffer.lock().unwrap();
                let mut output_index = 0;
                let mut last = 0;

                let delay_factor =
                    core_buf.len() as f64 / (output_buf.len() as f64 * resample_rate);

                // Delay compensation
                if delay_factor > 1.6 {
                    // Leave a tail of 0.1 to prevent crackling.
                    // The crackling occurs because there are less samples in the core buffer
                    // than in the output buffer, thus leaving the tail of the output empty.
                    let skipped_samples = ((delay_factor - 1.5) * output_buf.len() as f64) as usize;
                    core_buf.drain(..skipped_samples);

                    println!(
                        "AUDIO: Skipped {:05} samples. Delay factor: {:06} / {:06} = {}",
                        skipped_samples,
                        core_buf.len(),
                        output_buf.len(),
                        delay_factor
                    );
                }

                loop {
                    let sample_index = (output_index as f64 * resample_rate) as usize;

                    if output_index < output_buf.len() && sample_index < core_buf.len() {
                        output_buf[output_index] = core_buf[sample_index];
                        last = sample_index;
                    } else {
                        break;
                    }

                    output_index += 1;
                }

                // Remove used samples
                if last < core_buf.len() {
                    core_buf.drain(..=last);
                }
                true
            }
        })
        .unwrap();

        let gamepad_ids = HashSet::new();

        EmulatorState {
            emu,
            controllers,
            gamepad_ids,
            fb_copy,
            fb_image,
            fb_texture,
            fb_interlace_factor,
            audio_device,
            audio_stream,
            audio_buffer,
        }
    }

    pub fn update(&mut self, gilrs: &mut Gilrs) -> AppEvent {
        while let Some(Event { .. }) = gilrs.next_event() {}

        let mut keyboard_in_use = false;
        let mut registered_gamepad_iter = self.gamepad_ids.iter();

        for input in self.controllers.iter_mut() {
            let g_id = registered_gamepad_iter.next();

            if let Some(gamepad) = g_id.and_then(|g_id| gilrs.connected_gamepad(*g_id)) {
                update_input_port_with_gamepad(input, &gamepad);
            } else if !keyboard_in_use {
                keyboard_in_use = true;
                update_input_port_with_keyboard(input);
            }
        }

        // Check button combination to go back to menu
        if should_quit_game(gilrs) {
            return AppEvent::GoToMenu;
        }

        self.emu.run(self.controllers);
        self.update_framebuffer();
        self.update_audio_buffer().unwrap();

        // Gamepads
        for (g_id, _) in gilrs.gamepads() {
            self.gamepad_ids.insert(g_id);
        }

        AppEvent::Continue
    }

    fn update_framebuffer(&mut self) {
        let (fb_width, fb_height) = self.emu.framebuffer_size();
        let fb_pitch = self.emu.framebuffer_pitch();

        if fb_width != self.fb_image.width as usize || fb_height != self.fb_image.height as usize {
            self.resize_framebuffer(fb_width, fb_height, fb_pitch);

            info!(
                "Display mode changed: {:?} (width {}) (height {}) (pitch {} == {})",
                self.emu.pixel_format(),
                fb_width,
                fb_height,
                fb_pitch,
                fb_width * 4
            );
        }

        let pixfmt = self.emu.pixel_format();

        // Copy framebuffer
        let framebuffer_result = self.emu.peek_framebuffer(|fb: &[u8]| {
            let pixel_size = match pixfmt {
                PixelFormat::ARGB1555 => 2,
                PixelFormat::ARGB8888 => 4,
                PixelFormat::RGB565 => 2,
            };

            type ColorFn = Box<dyn Fn(&[u8]) -> (u8, u8, u8)>;
            let color_fn: ColorFn = match pixfmt {
                PixelFormat::ARGB1555 => unimplemented!(),
                PixelFormat::ARGB8888 => Box::new(|b| (b[2], b[1], b[0])),
                PixelFormat::RGB565 => Box::new(|b| pixels::rgb565to888(b[0], b[1])),
            };

            for y in 0..fb_height {
                for x in 0..fb_width {
                    let tex_index = (fb_width * y + x) * 4;
                    let fb_index = (fb_pitch * y) + (x * pixel_size);

                    if (fb_index + 2) >= fb.len() {
                        continue;
                    }

                    let (red, green, blue) = color_fn(&fb[fb_index..fb_index + pixel_size]);

                    self.fb_image.bytes[tex_index] = red; // R
                    self.fb_image.bytes[tex_index + 1] = green; // G
                    self.fb_image.bytes[tex_index + 2] = blue; // B
                    self.fb_image.bytes[tex_index + 3] = 0xFF; // A
                }
            }
        });

        match framebuffer_result {
            Err(RetroRsError::NoFramebufferError) => log::warn!("No framebuffer!"),
            Err(e) => panic!("{}", e),
            Ok(_) => (),
        }

        self.fb_texture.update(&self.fb_image);
    }

    fn update_audio_buffer(&mut self) -> Result<()> {
        self.emu.peek_audio_buffer(|b| {
            let mut buf = self.audio_buffer.lock().unwrap();
            buf.extend_from_slice(b);
        })?;

        Ok(())
    }

    fn resize_framebuffer(&mut self, width: usize, height: usize, pitch: usize) {
        self.fb_copy.resize(height * pitch, 0u8);
        self.fb_image = Image {
            bytes: [0x00, 0x00, 0x00, 0xFF].repeat(width * height),
            width: width as u16,
            height: height as u16,
        };
        self.fb_texture = Texture2D::from_image(&self.fb_image);
        self.fb_interlace_factor = (pitch - width) / 4;
    }

    pub fn render(&self, gilrs: &Gilrs) {
        clear_background(BLACK);

        let tex_width = self.fb_texture.width();
        let tex_height = self.fb_texture.height();
        let screen_width = screen_width();
        let screen_height = screen_height();

        let (width, height) = if (screen_width / screen_height) > (tex_width / tex_height) {
            ((tex_width * screen_height) / tex_height, screen_height)
        } else {
            (screen_width, (tex_height * screen_width) / tex_width)
        };

        draw_texture_ex(
            self.fb_texture,
            screen_width / 2. - width / 2.,
            screen_height / 2. - height / 2.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(width, height)),
                source: None,
                rotation: 0.0,
                flip_x: false,
                flip_y: false,
                pivot: None,
            },
        );

        let error_width = 100.0;
        let error_height = 50.0;

        // Gamepad disconnected warnings
        for (i, g_id) in self.gamepad_ids.iter().enumerate() {
            let gamepad = gilrs.gamepad(*g_id);

            let x = 20.0 + (error_width + 10.0) * i as f32;
            let y = screen_height - error_height;

            if !gamepad.is_connected() {
                println!("Gamepad {} ({}): Disconnected", g_id, gamepad.name());
                draw_rectangle(
                    x,
                    y,
                    error_width,
                    error_height,
                    Color::from_rgba(40, 0, 0, 240),
                );
                draw_text_ex(
                    gamepad.name(),
                    x,
                    y + error_height,
                    TextParams {
                        color: Color::from_rgba(255, 0, 0, 255),
                        ..Default::default()
                    },
                );
                draw_text_ex(
                    "Desconect.",
                    x,
                    y + error_height - 16.0,
                    TextParams {
                        color: Color::from_rgba(255, 0, 0, 255),
                        ..Default::default()
                    },
                );
            }
        }
    }

    pub fn snapshot(&self) -> Vec<u8> {
        let mut save_buffer = vec![0u8; self.emu.save_size()];
        self.emu.save(&mut save_buffer);
        save_buffer
    }
}

fn should_quit_game(gilrs: &Gilrs) -> bool {
    // Check for exit game keyboard and gamepad combinations
    // Start + Select + West = Quit game
    is_key_down(KeyCode::Escape)
        || gilrs.gamepads().fold(false, |should_quit, (_, g)| {
            should_quit
                || (g.is_pressed(Button::Select)
                    && g.is_pressed(Button::Start)
                    && g.is_pressed(Button::West))
        })
}
