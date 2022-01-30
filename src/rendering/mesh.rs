use std::{
    mem,
    sync::{Arc, RwLock},
};

use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::MeshUniform,
    prelude::{AddAsset, App, Commands, Component, Entity, GlobalTransform, Handle, Plugin, Query},
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin},
        render_component::ExtractComponentPlugin,
        render_resource::{
            Buffer, BufferAddress, BufferInitDescriptor, BufferUsages, PrimitiveTopology,
            VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
        RenderApp, RenderStage,
    },
};
use bytemuck::Pod;

pub struct SharedMeshPlugin;

impl Plugin for SharedMeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<SharedMesh>()
            .add_plugin(ExtractComponentPlugin::<Handle<SharedMesh>>::default())
            .add_plugin(RenderAssetPlugin::<SharedMesh>::default());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_system_to_stage(RenderStage::Extract, extract_shared_meshes);
        }
    }
}

impl RenderAsset for SharedMesh {
    type ExtractedAsset = SharedMesh;
    type PreparedAsset = SharedMesh;

    type Param = (
        SRes<RenderDevice>,
        //SRes<ModelDrawMaterialPipeline<Self>>, // <- pipeline
        //SRes<RenderAssets<Image>>,
    );
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        _render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        println!(" asset mesh");
        Ok(extracted_asset)
    }
}

// todo: remove T type

#[derive(Clone, Debug, TypeUuid)]
#[uuid = "48a9c363-1124-4113-890e-199d81b00281"]
pub struct SharedMesh {
    pub name: String,          // <-----------
    pub vertex_buffer: Buffer, // <-----------
    pub index_buffer: Buffer,  // <-----------
    pub vertex_size: usize,
    pub mesh_handel: Vec<Arc<RwLock<SubMeshHandel>>>, // <-----------
    pub test_max_index: u32,
    pub test_index: u32,
    pub test_vertex: u32,
    pub vertex_buff_size: usize,
    pub index_buff_size: usize,

    removed_a_handel: bool,
    pub primitive_topology: PrimitiveTopology,
}

impl SharedMesh {
    pub fn new<T: Pod>(
        name: String,
        device: &RenderDevice,
        // (vertex, index)
        buffer_sizes: (usize, usize),
        primitive_topology: PrimitiveTopology,
    ) -> Self {
        let vertex_size = mem::size_of::<T>();
        let vertex_buff_size = buffer_sizes.0 * vertex_size;
        let index_buff_size = buffer_sizes.1 * mem::size_of::<u32>();

        println!(
            "vertex_buffer bytes: {},  megabytes: {}, GBs: {}",
            vertex_buff_size,
            vertex_buff_size as f64 * 0.000001,
            vertex_buff_size as f64 * 0.000001 * 0.00097656
        );
        println!(
            "index_buffer bytes: {},  megabytes: {}, GBs: {}",
            index_buff_size,
            index_buff_size as f64 * 0.000001,
            index_buff_size as f64 * 0.000001 * 0.00097656
        );
        println!(
            "total_buffer bytes: {},  megabytes: {}, GBs: {}",
            index_buff_size + vertex_buff_size,
            (index_buff_size + vertex_buff_size) as f64 * 0.000001,
            (index_buff_size + vertex_buff_size) as f64 * 0.000001 * 0.00097656
        );

        let vertex_buffer = vec![0 as u8; buffer_sizes.0 as usize * vertex_size as usize];
        let index_buffer = vec![0 as u8; buffer_sizes.1 as usize * mem::size_of::<u32>()];

        Self {
            name,
            vertex_buffer: device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("shared Vertex Buffer"),
                contents: bytemuck::cast_slice(vertex_buffer.as_slice()),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            }),
            index_buffer: device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("shared Index Buffer"),
                contents: bytemuck::cast_slice(index_buffer.as_slice()),
                usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            }),
            vertex_size,
            mesh_handel: vec![],
            test_max_index: 0,
            test_index: 0,
            test_vertex: 0,
            index_buff_size,
            vertex_buff_size,
            removed_a_handel: false,

            primitive_topology,
        }
    }

    /// gets a new handel set to the provided mesh data
    pub fn get_handel<T: Pod>(
        &mut self,

        vertex_buffer: &[T],
        index_buffer: &[u32],
        queue: &RenderQueue,
    ) -> Arc<RwLock<SubMeshHandel>> {
        let handel = Arc::new(RwLock::new(SubMeshHandel {
            vertex_start: 0,
            index_start: 0,
            vertex_length: vertex_buffer.len() as u32,
            index_length: index_buffer.len() as u32,
        }));
        self.set_handel(handel, vertex_buffer, index_buffer, queue)
    }

    fn set_handel<T: Pod>(
        &mut self,
        handel: Arc<RwLock<SubMeshHandel>>,
        vertex_buffer: &[T],
        index_buffer: &[u32],
        queue: &RenderQueue,
    ) -> Arc<RwLock<SubMeshHandel>> {
        if self.mesh_handel.is_empty() {
            queue.write_buffer(
                &self.vertex_buffer,
                (0) as BufferAddress,
                bytemuck::cast_slice(&vertex_buffer),
            );
            queue.write_buffer(
                &self.index_buffer,
                (0) as BufferAddress,
                bytemuck::cast_slice(&index_buffer),
            );
            self.mesh_handel.push(handel.clone());
            return handel.clone();
        } else {
            let mut w_handel = handel.write().unwrap();

            //    if self.removed_a_handel {
            self.mesh_handel.sort_by(|a, b| {
                a.read()
                    .unwrap()
                    .vertex_start
                    .cmp(&b.read().unwrap().vertex_start)
            });
            //    }
            let mut mh_iter = self.mesh_handel.iter();
            let mut hit_once = false;

            'v: loop {
                match mh_iter.next() {
                    Some(h) => {
                        let r_h = h.read().unwrap();

                        if SubMeshHandel::overlap(
                            w_handel.vertex_start,
                            w_handel.vertex_start + w_handel.vertex_length,
                            r_h.vertex_start,
                            r_h.vertex_start + r_h.vertex_length,
                        ) {
                            w_handel.vertex_start = r_h.vertex_start + r_h.vertex_length;
                            hit_once = true;
                        } else if hit_once {
                            queue.write_buffer(
                                &self.vertex_buffer,
                                (self.vertex_size * w_handel.vertex_start as usize)
                                    as BufferAddress,
                                bytemuck::cast_slice(&vertex_buffer),
                            );
                            break 'v;
                        }
                    }
                    None => {
                        queue.write_buffer(
                            &self.vertex_buffer,
                            (self.vertex_size * w_handel.vertex_start as usize) as BufferAddress,
                            bytemuck::cast_slice(&vertex_buffer),
                        );
                        break 'v;
                    }
                }
            }
            //if self.removed_a_handel {
            self.mesh_handel.sort_by(|a, b| {
                a.read()
                    .unwrap()
                    .index_start
                    .cmp(&b.read().unwrap().index_start)
            });
            // }

            let mut mh_iter = self.mesh_handel.iter();
            hit_once = false;
            'i: loop {
                match mh_iter.next() {
                    Some(h) => {
                        let r_h = h.read().unwrap();
                        if w_handel.index_start > r_h.index_start + r_h.index_length {
                            continue;
                        }

                        if SubMeshHandel::overlap(
                            w_handel.index_start,
                            w_handel.index_start + w_handel.index_length,
                            r_h.index_start,
                            r_h.index_start + r_h.index_length,
                        ) {
                            w_handel.index_start = r_h.index_start + r_h.index_length;
                            hit_once = true;
                        } else if hit_once {
                            queue.write_buffer(
                                &self.index_buffer,
                                (mem::size_of::<u32>() as u32 * w_handel.index_start)
                                    as BufferAddress,
                                bytemuck::cast_slice(&index_buffer),
                            );
                            break 'i;
                        }
                    }
                    None => {
                        queue.write_buffer(
                            &self.index_buffer,
                            (mem::size_of::<u32>() as u32 * w_handel.index_start) as BufferAddress,
                            bytemuck::cast_slice(&index_buffer),
                        );
                        break 'i;
                    }
                }
            }
            //-----------------------------------
            self.removed_a_handel = false;
            if (w_handel.index_start + w_handel.index_length) > self.test_max_index {
                self.test_max_index = (w_handel.index_start + w_handel.index_length);
            }
            //  drop(w_handel);
            self.mesh_handel.push(handel.clone());
            return handel.clone();
        }
    }

    pub fn remove_handel(&mut self, handel_to_remove: &Arc<RwLock<SubMeshHandel>>) -> bool {
        let handel_r = handel_to_remove.read().unwrap();
        let mut index_ = 0;
        let mut to_remove = false;
        for (i, mh) in self.mesh_handel.iter().enumerate() {
            let mh_r = mh.read().unwrap();
            if mh_r.vertex_start == handel_r.vertex_start {
                index_ = i;
                to_remove = true;

                break;
            }
            if mh_r.index_start == handel_r.index_start {
                index_ = i;
                to_remove = true;

                break;
            }
        }
        if to_remove {
            self.mesh_handel.remove(index_);
            self.removed_a_handel = true;
            return true;
        }

        return false;
    }

    /// updates the underlying mesh data of a handel
    pub fn update_model<T: Pod>(
        &mut self,
        handel: Arc<RwLock<SubMeshHandel>>,
        vertex_buffer: &[T],
        index_buffer: &[u32],
        queue: &RenderQueue,
    ) {
        if self.remove_handel(&handel) {
            if let Ok(mut val) = handel.write() {
                val.vertex_length = vertex_buffer.len() as u32;
                val.index_length = index_buffer.len() as u32;
            }
            self.set_handel(handel, vertex_buffer, index_buffer, queue);
        }
    }

    pub fn print_size(&self) {
        println!(
            "vertex_buffer bytes: {},  megabytes: {}, GBs: {:.4}",
            self.vertex_buff_size,
            self.vertex_buff_size as f64 * 0.000001,
            self.vertex_buff_size as f64 * 0.000001 * 0.00097656
        );
        println!(
            "index_buffer bytes: {},  megabytes: {}, GBs: {:.4}",
            self.index_buff_size,
            self.index_buff_size as f64 * 0.000001,
            self.index_buff_size as f64 * 0.000001 * 0.00097656
        );
        println!(
            "total_buffer bytes: {},  megabytes: {}, GBs: {:.4}",
            self.index_buff_size + self.vertex_buff_size,
            (self.index_buff_size + self.vertex_buff_size) as f64 * 0.000001,
            (self.index_buff_size + self.vertex_buff_size) as f64 * 0.000001 * 0.00097656
        );

        let mut used_index_bytes = 0;
        let mut used_vertex_bytes = 0;
        let mut empty_space_bytes = 0;

        // todo: print the in use byte size, include the dead space inbetwen handel ranges

        for smh in self.mesh_handel.iter() {
            let r_smh = smh.read().unwrap();

            used_vertex_bytes += r_smh.vertex_length as usize * self.vertex_size;
            used_index_bytes += r_smh.index_length as usize * mem::size_of::<u32>();
        }
        println!(
            "used vertex bytes: {},  megabytes: {:.4}, GBs: {:.4}",
            used_vertex_bytes,
            used_vertex_bytes as f64 * 0.000001,
            used_vertex_bytes as f64 * 0.000001 * 0.00097656
        );
        println!(
            "used index bytes: {},  megabytes: {:.4}, GBs: {:.4}",
            used_index_bytes,
            used_index_bytes as f64 * 0.000001,
            used_index_bytes as f64 * 0.000001 * 0.00097656
        );
    }
    pub fn get_size(&self) -> (String, String, String, String) {
        let vb = format!(
            "vertex_buffer bytes: {},  megabytes: {:.4}, GBs: {:.4}",
            self.vertex_buff_size,
            self.vertex_buff_size as f64 * 0.000001,
            self.vertex_buff_size as f64 * 0.000001 * 0.00097656
        );
        let ib = format!(
            "index_buffer bytes: {},  megabytes: {:.4}, GBs: {:.4}",
            self.index_buff_size,
            self.index_buff_size as f64 * 0.000001,
            self.index_buff_size as f64 * 0.000001 * 0.00097656
        );
        let tb = format!(
            "total_buffer bytes: {},  megabytes: {:.4}, GBs: {:.4}",
            self.index_buff_size + self.vertex_buff_size,
            (self.index_buff_size + self.vertex_buff_size) as f64 * 0.000001,
            (self.index_buff_size + self.vertex_buff_size) as f64 * 0.000001 * 0.00097656
        );

        let mut used_vertex_bytes = 0;
        let mut empty_space_bytes = 0;

        // todo: print the in use byte size, include the dead space inbetwen handel ranges

        for smh in self.mesh_handel.iter() {
            let r_smh = smh.read().unwrap();

            used_vertex_bytes += r_smh.vertex_length as usize * self.vertex_size;
        }
        let vb_used = format!(
            "used vertex bytes: {},  megabytes: {:.4}, GBs: {:.4}",
            used_vertex_bytes,
            used_vertex_bytes as f64 * 0.000001,
            used_vertex_bytes as f64 * 0.000001 * 0.00097656
        );
        (vb, ib, tb, vb_used)
    }

    /// for debugging.
    /// to test if buffer writhing is working.
    pub fn flap_push<T: Pod>(
        &mut self,
        vertex_buffer: &[T],
        index_buffer: &[u32],
        queue: &RenderQueue,
    ) -> Arc<RwLock<SubMeshHandel>> {
        let mut handel = Arc::new(RwLock::new(SubMeshHandel {
            vertex_start: 0,
            index_start: 0,
            vertex_length: vertex_buffer.len() as u32,
            index_length: index_buffer.len() as u32,
        }));

        println!("flat push ");
        queue.write_buffer(
            &self.vertex_buffer,
            (mem::size_of::<T>() * self.test_vertex as usize) as BufferAddress,
            bytemuck::cast_slice(&vertex_buffer),
        );

        queue.write_buffer(
            &self.index_buffer,
            (mem::size_of::<u32>() as u32 * self.test_index) as BufferAddress,
            bytemuck::cast_slice(&index_buffer),
        );

        self.test_index += index_buffer.len() as u32;
        self.test_vertex += vertex_buffer.len() as u32;

        //if (w_handel.index_start + w_handel.index_length) > self.test_max_index {
        //   self.test_max_index = (w_handel.index_start + w_handel.index_length);
        // }
        //  drop(w_handel);
        handel
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
pub struct SubMeshHandel {
    pub vertex_start: u32,
    pub vertex_length: u32,
    pub index_start: u32,
    pub index_length: u32,
}

impl SubMeshHandel {
    pub fn overlap(w_start: u32, w_end: u32, r_start: u32, r_end: u32) -> bool {
        if w_start >= r_start && w_start < r_end {
            // w start is inside r
            return true;
        } else if w_end > r_start && w_end <= r_end {
            // w end is inside r
            return true;
        } else if w_start <= r_start && w_end >= r_end {
            // r is inside w
            return true;
        } //else if w_start == r_start {
          //    return true;
          // }
          // if w_end == r_end {
          //      return true;
          //   }
        return false;
    }
}

/*
impl ExtractComponent for SharedMesh {
    type Query = &'static SharedMesh;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}

// ----------------------------------------------------------------
*/
pub fn extract_shared_meshes(
    mut commands: Commands,
    query: Query<(Entity, &GlobalTransform, &Handle<SharedMesh>)>,
) {
    let mut not_caster_values = Vec::new();
    for (entity, transform, _shared_mesh) in query.iter() {
        let transform = transform.compute_matrix();
        not_caster_values.push((
            entity,
            (MeshUniform {
                flags: 0,
                transform,
                inverse_transpose_model: transform.inverse().transpose(),
            },),
        ));
    }

    // add mesh uniform to entity
    commands.insert_or_spawn_batch(not_caster_values);
}

// --------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ChunkMeshvertex {
    pub pos: [f32; 3],
    pub normal: [i8; 4],
    pub color: [u8; 4],
    pub uv: [u8; 4],
}
impl ChunkMeshvertex {
    pub fn new(
        position: [f32; 3],
        normal: [f32; 3],
        color: [f32; 3],
        uv_0: [f32; 2],
        index: u16,
    ) -> Self {
        ChunkMeshvertex {
            pos: position,
            normal: [
                (normal[0] * 128.0) as i8,
                (normal[1] * 128.0) as i8,
                (normal[2] * 128.0) as i8,
                0,
            ],
            color: [
                (color[0] * 255.0) as u8,
                (color[1] * 255.0) as u8,
                (color[2] * 255.0) as u8,
                (0.0 * 255.0) as u8,
            ],
            uv: [
                (uv_0[0] * 255.0) as u8,
                (uv_0[1] * 255.0) as u8,
                // pack
                ((index & 0xff_00) >> 8) as u8,
                (index & 0x00_ff) as u8,
            ],
        }
    }
    pub fn desc() -> VertexBufferLayout {
        use std::mem;
        VertexBufferLayout {
            array_stride: mem::size_of::<ChunkMeshvertex>() as BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: VertexStepMode::Vertex,
            attributes: vec![
                VertexAttribute {
                    // pos
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                // norm
                VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Snorm8x4,
                },
                // col
                VertexAttribute {
                    offset: (mem::size_of::<[f32; 3]>() + mem::size_of::<[u8; 4]>())
                        as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Unorm8x4,
                },
                // uv + index
                VertexAttribute {
                    offset: (mem::size_of::<[f32; 3]>() + mem::size_of::<[u8; 8]>())
                        as BufferAddress,
                    shader_location: 3,
                    format: VertexFormat::Uint8x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineMeshvertex {
    pub pos: [f32; 3],
    //pub normal: [i8; 4],
    pub color: [u8; 4],
    //pub uv: [u8; 4],
}
impl LineMeshvertex {
    pub fn new(position: [f32; 3], color: [f32; 4]) -> Self {
        LineMeshvertex {
            pos: position,

            color: [
                (color[0] * 255.0) as u8,
                (color[1] * 255.0) as u8,
                (color[2] * 255.0) as u8,
                (color[3] * 255.0) as u8,
            ],
        }
    }
    pub fn desc() -> VertexBufferLayout {
        VertexBufferLayout {
            array_stride: mem::size_of::<LineMeshvertex>() as BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: VertexStepMode::Vertex,
            attributes: vec![
                VertexAttribute {
                    // pos
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                // col
                VertexAttribute {
                    offset: (mem::size_of::<[f32; 3]>()) as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Unorm8x4,
                },
            ],
        }
    }
}
