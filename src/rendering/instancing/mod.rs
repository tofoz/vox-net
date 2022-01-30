use std::sync::{Arc, RwLock};

use bevy::{
    prelude::{App, Commands, Component, Entity, Plugin, Query, Res},
    render::{
        render_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::{
            Buffer, BufferAddress, BufferInitDescriptor, BufferUsages, VertexAttribute,
            VertexBufferLayout, VertexFormat, VertexStepMode,
        },
        renderer::RenderDevice,
        RenderApp, RenderStage,
    },
};
use prism_math::{Mat4, Quat, Vec3};

use super::mesh::SubMeshHandel;

pub struct InstancePlugin;

impl Plugin for InstancePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<InstanceList>::default());
        app.sub_app_mut(RenderApp)
            .add_system_to_stage(RenderStage::Prepare, prepare_instance_buffers);
    }
}

fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &InstanceList)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, instance_data) in query.iter() {
        let mut raw_list = vec![];
        for i in instance_data.instance_list.iter() {
            raw_list.push(i.to_raw())
        }

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instance data buffer"),
            contents: bytemuck::cast_slice(raw_list.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            length: instance_data.instance_list.len(),
        });
    }
}

#[derive(Component)]
pub struct InstanceList {
    pub instance_list: Vec<Instance>,
}

impl ExtractComponent for InstanceList {
    type Query = &'static InstanceList;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        InstanceList {
            instance_list: item.instance_list.clone(),
        }
    }
}

#[derive(Clone, Copy)]

pub struct Instance {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub color: [f32; 4],
}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (Mat4::from_scale_rotation_translation(
                self.scale,
                self.rotation,
                self.position,
            ))
            .to_cols_array_2d()
            .into(),
            color: self.color,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
    color: [f32; 4],
}
impl InstanceRaw {
    pub fn desc() -> VertexBufferLayout {
        use std::mem;
        VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                // transform mat
                VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as BufferAddress,
                    shader_location: 6,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as BufferAddress,
                    shader_location: 7,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as BufferAddress,
                    shader_location: 8,
                    format: VertexFormat::Float32x4,
                },
                // color rgba
                VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as BufferAddress,
                    shader_location: 9,
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}

// gpu
#[derive(Component)]
pub struct InstanceBuffer {
    pub buffer: Buffer,
    pub length: usize,
}

// ---------------------------------------------------------------------------------------------------------------

pub struct InstanceModelPlugin;

impl Plugin for InstanceModelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<ModelInstanceList>::default());
        app.sub_app_mut(RenderApp)
            .add_system_to_stage(RenderStage::Prepare, prepare_model_instance_buffers)
            // model instance to indirect needs to run after prepare_model_instance_buffer
            .add_system_to_stage(RenderStage::Queue, model_instance_to_draw_indirect_list);
    }
}

fn prepare_model_instance_buffers(
    mut commands: Commands,
    mut query: Query<(Entity, &mut ModelInstanceList)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, mut instance_data) in query.iter_mut() {
        let mut raw_list = vec![];
        for (i, inst) in instance_data.instance_list.iter_mut().enumerate() {
            raw_list.push(inst.instance.to_raw());
            inst.inst_index = i as u32;
        }

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("ModelInstance data buffer"),
            contents: bytemuck::cast_slice(raw_list.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            length: raw_list.len(),
        });
    }
}

fn model_instance_to_draw_indirect_list(
    mut commands: Commands,
    query: Query<(Entity, &ModelInstanceList)>,
) {
    for (entity, instance_data) in query.iter() {
        let mut inderect_l = DrawIndexedIndirectList {
            draw_indirect: vec![],
        };
        for model_instance in instance_data.instance_list.iter() {
            let r_h = model_instance.mesh.read().unwrap();
            inderect_l.draw_indirect.push(DrawIndexedIndirect {
                vertex_count: r_h.index_length,
                instance_count: 1,
                base_index: r_h.index_start,
                vertex_offset: r_h.vertex_start as i32,
                base_instance: model_instance.inst_index as u32,
            });
        }
        commands.get_or_spawn(entity).insert(inderect_l);
    }
}

#[derive(Clone, Component)]
pub struct ModelInstance {
    pub mesh: Arc<RwLock<SubMeshHandel>>,
    pub instance: Instance,

    pub inst_index: u32,
}

#[derive(Clone, Component)]
pub struct ModelInstanceList {
    pub instance_list: Vec<ModelInstance>,
}

impl ExtractComponent for ModelInstanceList {
    type Query = &'static ModelInstanceList;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}

#[derive(Clone, Component)]
pub struct DrawIndexedIndirectList {
    pub draw_indirect: Vec<DrawIndexedIndirect>,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct DrawIndexedIndirect {
    /// The number of vertices to draw.
    pub vertex_count: u32,
    /// The number of instances to draw.
    pub instance_count: u32,
    /// The base index within the index buffer.
    pub base_index: u32,
    /// The value added to the vertex index before indexing into the vertex buffer.
    pub vertex_offset: i32,
    /// The instance ID of the first instance to draw.
    pub base_instance: u32,
}
