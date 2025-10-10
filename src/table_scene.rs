use basic_3d::camera::{Camera3d, FlyCameraSystem};
use cgmath::{Deg, Rad};
use engine_core::{SceneBuilder, assets::obj::spawn_mesh_from_obj};
use render::{
    mesh::Mesh,
    vertex::{MeshVertex, ScreenQuadVertex},
    winit::dpi::PhysicalSize,
};

use crate::def_render::deferred_3d_pipeline;

pub fn load_scene(win_size: PhysicalSize<u32>) -> SceneBuilder {
    SceneBuilder::new()
        .register_component::<Camera3d>()
        .register_component::<Mesh<MeshVertex>>()
        .register_component::<Mesh<ScreenQuadVertex>>()
        .spawn_component(Mesh::new_screen_quad_u32())
        .spawn_component(Camera3d::new_basic(
            [0.0, 0.0, 0.0],
            Rad(0.0),
            Rad(0.0),
            win_size,
        ))
        .spawn(spawn_mesh_from_obj(
            "/Users/jareddewey/Documents/Coding/fun-lights/res/table.obj",
            [0.0, 0.0, 0.0],
        ))
        .register_system(FlyCameraSystem {
            speed: 5.0,
            rot_speed: Deg(90.0).into(),
        })
        .prep_render_pipeline(&deferred_3d_pipeline)
}
