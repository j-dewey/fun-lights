use basic_3d::camera::Camera3d;
use cgmath::{Rad, Vector3};
use engine_core::SceneBuilder;
use render::{mesh::Mesh, vertex::MeshVertex, winit::dpi::PhysicalSize};

use crate::{debug, renderable::init_renderable, visual::load_render_pipeline};

pub fn load_scene(win_size: PhysicalSize<u32>) -> SceneBuilder {
    SceneBuilder::new()
        .register_component::<Camera3d>()
        .register_component::<Mesh<MeshVertex>>()
        .spawn_component(Camera3d::new_basic(
            [0.0, 0.0, 0.0],
            Rad(0.0),
            Rad(0.0),
            win_size,
        ))
        .spawn(init_renderable(
            "/Users/jareddewey/Documents/Coding/fun-lights/res/table.obj",
            Vector3::new(10.0, 0.0, 0.0),
        ))
        .register_system(debug::CameraDisplayer {})
        .register_system(debug::MeshDisplayer {})
        .prep_render_pipeline(&load_render_pipeline)
}
