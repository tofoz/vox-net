use bevy::{
    core_pipeline::Opaque3d,
    ecs::system::{lifetimeless::*, SystemParamItem},
    pbr::{MeshPipeline, MeshPipelineKey, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::*,
        renderer::RenderDevice,
        view::{ExtractedView, Msaa},
        RenderApp, RenderStage,
    },
};

use super::{
    instancing::*,
    mesh::{ChunkMeshvertex, SharedMesh, SharedMeshPlugin},
    old_chunk_material::{ChunkMaterialPlugin, MyMaterial},
};

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(SharedMeshPlugin);
        app.add_plugin(InstanceModelPlugin);
        app.add_plugin(ChunkMaterialPlugin);

        app.sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawShardMesh>()
            .init_resource::<CustomPipeline>()
            .init_resource::<SpecializedPipelines<CustomPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_custom);
    }
}

fn queue_custom(
    transparent_3d_draw_functions: Res<DrawFunctions<Opaque3d>>,
    materials: Res<RenderAssets<MyMaterial>>,
    custom_pipeline: Res<CustomPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedPipelines<CustomPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    material_meshes: Query<
        (Entity, &Handle<MyMaterial>),
        (With<ModelInstanceList>, With<SharedMesh<ChunkMeshvertex>>),
    >,

    mut views: Query<(&ExtractedView, &mut RenderPhase<Opaque3d>)>,
) {
    let draw_custom = transparent_3d_draw_functions
        .read()
        .get_id::<DrawShardMesh>()
        .unwrap();

    let key = MeshPipelineKey::from_msaa_samples(msaa.samples)
        | MeshPipelineKey::from_primitive_topology(PrimitiveTopology::TriangleList);
    let pipeline = pipelines.specialize(&mut pipeline_cache, &custom_pipeline, key);

    for (view, mut transparent_phase) in views.iter_mut() {
        for (entity, material_handle) in material_meshes.iter() {
            if materials.contains_key(material_handle) {
                transparent_phase.add(Opaque3d {
                    entity,
                    pipeline,
                    draw_function: draw_custom,
                    distance: 0.0, // its instanced therefor there is no one distance
                });
            }
        }
    }
}

pub struct CustomPipeline {
    pub shader: Handle<Shader>,
    pub material_layout: BindGroupLayout,
    pub mesh_pipeline: MeshPipeline,
}

impl FromWorld for CustomPipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let asset_server = world.get_resource::<AssetServer>().unwrap();

        let shader = asset_server.load("shaders/instancing.wgsl");

        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            <Vec4 as std140::AsStd140>::std140_size_static() as u64,
                        ),
                    },
                    count: None,
                }, // Base Color Texture
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                    },
                    count: None,
                },
                // Base Color Texture Sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: None,
        });

        let mesh_pipeline = world.get_resource::<MeshPipeline>().unwrap().clone();
        CustomPipeline {
            shader,
            material_layout,
            mesh_pipeline,
        }
    }
}

impl SpecializedPipeline for CustomPipeline {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut descriptor = self.mesh_pipeline.specialize(key);
        descriptor.vertex.shader = self.shader.clone();
        //
        descriptor.vertex.buffers = vec![ChunkMeshvertex::desc()];
        //descriptor.vertex.buffers.push(ChunkMeshvertex::desc());
        descriptor.vertex.buffers.push(InstanceRaw::desc());
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
            self.material_layout.clone(), // <------- texture layout
        ]);

        let mut shader_defs = vec![];
        if false {
            shader_defs.push("DEBUG_UV".to_string());
        }
        if true {
            shader_defs.push("IS_LIGHTING".to_string());
        }

        if true {
            shader_defs.push("COLOR_TEXTURE".to_string());
        }

        descriptor.vertex.shader_defs.append(&mut shader_defs);
        if let Some(frag) = &mut descriptor.fragment {
            frag.shader_defs.append(&mut shader_defs);
        }

        descriptor
    }
}

//--------------------------------------------------------

type DrawShardMesh = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetCustomMaterialBindGroup<2>,
    SetMeshBindGroup<1>,
    DrawMeshInstanced,
);

struct SetCustomMaterialBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetCustomMaterialBindGroup<I> {
    type Param = (
        SRes<RenderAssets<MyMaterial>>,
        SQuery<Read<Handle<MyMaterial>>>,
    );
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (materials, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material_handle = query.get(item).unwrap();
        let material = materials.into_inner().get(material_handle).unwrap();
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawMeshInstanced;
impl EntityRenderCommand for DrawMeshInstanced {
    type Param = (
        SQuery<Read<SharedMesh<ChunkMeshvertex>>>,
        SQuery<Read<DrawIndexedIndirectList>>,
        SQuery<Read<InstanceBuffer>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (s_mesh, draw_indirect_query, instance_buffer_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let shard_mesh = s_mesh.get(item).unwrap();
        let instance_buffer = instance_buffer_query.get(item).unwrap();
        let draw_indirect_list = draw_indirect_query.get(item).unwrap();

        pass.set_vertex_buffer(0, shard_mesh.vertex_buffer.slice(..)); // copying a mesh buffer could be problume?
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));
        pass.set_index_buffer(shard_mesh.index_buffer.slice(..), 0, IndexFormat::Uint32);

        for i in draw_indirect_list.draw_indirect.iter() {
            pass.draw_indexed(
                i.base_index..(i.base_index + i.vertex_count),
                i.vertex_offset,
                i.base_instance..(i.base_instance + i.instance_count),
            );
        }

        RenderCommandResult::Success
    }
}
