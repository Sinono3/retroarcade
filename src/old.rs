mod buttons;
mod colors;
mod emulator;
mod error;
mod fb_to_image;

use std::path::Path;

use buttons::*;
use colors::*;
use emulator::*;
use error::*;
use fb_to_image::*;

use ::pixels::{Pixels, SurfaceTexture};
use anyhow::Result;
use log::error;
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

fn main() -> Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size = LogicalSize::new(WIDTH as f64 * 3.0, HEIGHT as f64 * 3.0);
        WindowBuilder::new()
            .with_title("Conway's Game of Life")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    let mut emu = Emulator::create(
        Path::new("cores/fceumm_libretro.so"),
        Path::new("roms/mario.nes"),
    );

    emu.run([Buttons::new(), Buttons::new()]);
    emu.reset();

    let (width, height) = emu.framebuffer_size();
    window.set_inner_size(LogicalSize::new(width as u32, height as u32));
    pixels.resize_buffer(width as u32, height as u32);

    let mut framebuffer = vec![0u8; width * height * 3];
    let mut save_buffer = vec![0u8; emu.save_size()];

    event_loop.run(move |event, _, control_flow| {
        // The one and only event that winit_input_helper doesn't have for us...
        if let Event::RedrawRequested(_) = event {
            // Copy framebuffer
            emu.copy_framebuffer_rgb888(&mut framebuffer).unwrap();

            let frame = pixels.get_frame();

            for (pixel, emu_pixel) in frame.chunks_exact_mut(4).zip(framebuffer.chunks_exact(3)) {
                pixel[0] = emu_pixel[0]; // R
                pixel[1] = emu_pixel[1]; // G
                pixel[2] = emu_pixel[2]; // B
                pixel[3] = 0xFF; // A
            }

            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // For everything else, for let winit_input_helper collect events to build its state.
        // It returns `true` when it is time to update our game state and request a redraw.
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }
            // Resize the window
            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
            }

            // Save/load
            if input.key_pressed(VirtualKeyCode::S) {
                emu.save(&mut save_buffer);
            } else if input.key_pressed(VirtualKeyCode::L) {
                emu.load(&save_buffer);
            }

            let buttons = Buttons::new()
                .up(input.key_held(VirtualKeyCode::Up))
                .down(input.key_held(VirtualKeyCode::Down))
                .left(input.key_held(VirtualKeyCode::Left))
                .right(input.key_held(VirtualKeyCode::Right))
                .a(input.key_held(VirtualKeyCode::S))
                .b(input.key_held(VirtualKeyCode::D))
                .x(input.key_held(VirtualKeyCode::A))
                .y(input.key_held(VirtualKeyCode::W))
                .start(input.key_held(VirtualKeyCode::Return))
                .select(input.key_held(VirtualKeyCode::Escape));

            emu.run([buttons, Buttons::new()]);
            window.request_redraw();
        }
    });
}
