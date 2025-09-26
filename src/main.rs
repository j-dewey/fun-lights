#![feature(iter_array_chunks)]
use engine_core::{App, input};
use render::winit::{event_loop::EventLoop, window::WindowBuilder};

//mod debug;
mod def_render;
mod renderable;
mod table_scene;
mod visual;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let win_size = window.inner_size();

    App::new(&window).run(table_scene::load_scene(win_size), event_loop);
}
