use std::any::TypeId;

use basic_3d::camera::Camera3d;
use engine_core::{
    MASTER_THREAD,
    system::{System, SystemId, SystemInterface, UpdateResult},
};
use frosty_alloc::{AllocId, FrostyAllocatable};
use render::{mesh::Mesh, vertex::MeshVertex};

pub struct MeshDisplayer {}

impl SystemInterface for MeshDisplayer {
    fn alloc_id(&self) -> AllocId {
        Mesh::<MeshVertex>::id()
    }

    fn dependencies() -> Vec<engine_core::system::SystemId>
    where
        Self: Sized,
    {
        vec![]
    }

    fn id() -> SystemId
    where
        Self: Sized,
    {
        SystemId(TypeId::of::<Self>())
    }

    fn start_update(&self, objs: engine_core::query::Query<u8>) -> UpdateResult {
        unsafe { self.update(objs.cast()) }
    }
}

impl System for MeshDisplayer {
    type Interop = Mesh<MeshVertex>;

    fn update(&self, mut objs: engine_core::query::Query<Self::Interop>) -> UpdateResult {
        println!("\nNew frame!");
        while let Some(mesh) = objs.next(MASTER_THREAD) {
            /*for v in &mesh.as_ref().verts {
                println!("v at {:?}", v.world_pos);
            }*/
            println!("{:?}", mesh.as_ref().verts.as_ptr());
        }
        panic!();
        UpdateResult::Skip
    }
}

pub struct CameraDisplayer {}

impl SystemInterface for CameraDisplayer {
    fn alloc_id(&self) -> AllocId {
        Camera3d::id()
    }

    fn dependencies() -> Vec<engine_core::system::SystemId>
    where
        Self: Sized,
    {
        vec![]
    }

    fn id() -> SystemId
    where
        Self: Sized,
    {
        SystemId(TypeId::of::<Self>())
    }

    fn start_update(&self, objs: engine_core::query::Query<u8>) -> UpdateResult {
        unsafe { self.update(objs.cast()) }
    }
}

impl System for CameraDisplayer {
    type Interop = Camera3d;

    fn update(&self, mut objs: engine_core::query::Query<Self::Interop>) -> UpdateResult {
        println!("\nNew frame!");
        while let Some(mesh) = objs.next(MASTER_THREAD) {
            let cam = mesh.as_ref();
            println!("{:?}", cam);
        }

        UpdateResult::Skip
    }
}
