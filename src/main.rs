#![feature(iter_array_chunks)]
use basic_3d::camera::FlyCameraSystem;
use engine_core::{App, input};
use render::winit::{event_loop::EventLoop, window::WindowBuilder};

//mod debug;
mod def_render;
mod table_scene;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let win_size = window.inner_size();

    // input init failure just means input was already
    // init, so can just ignore that.
    unsafe {
        #[allow(unused_must_use)]
        input::init_input(win_size);
        FlyCameraSystem::load_controls().expect("Failed to init input");
    }

    App::new(&window).run(table_scene::load_scene(win_size), event_loop);
}
