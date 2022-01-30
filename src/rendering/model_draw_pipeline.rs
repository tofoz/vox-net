use std::{collections::HashMap, marker::PhantomData};

use bevy::{
    asset::HandleId,
    core_pipeline::{AlphaMask3d, Opaque3d, Transparent3d},
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    pbr::{
        AlphaMode, MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup,
        SetMeshViewBindGroup, SpecializedMaterial,
    },
    prelude::{
        AddAsset, App, AssetServer, Commands, Component, Entity, FromWorld, Handle, Msaa, Plugin,
        Query, Res, ResMut, Shader, With, World,
    },
    reflect::TypeUuid,
    render::{
        render_asset::{RenderAssetPlugin, RenderAssets},
        render_component::ExtractComponentPlugin,
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroupLayout, IndexFormat, PrimitiveTopology, RenderPipelineCache,
            RenderPipelineDescriptor, SpecializedPipeline, SpecializedPipelines,
        },
        renderer::RenderDevice,
        view::VisibleEntities,
        RenderApp, RenderStage,
    },
};
use prism_math::Vec4;

use super::{
    instancing::{DrawIndexedIndirectList, InstanceBuffer, ModelInstanceList},
    mesh::SharedMesh,
};

// note: alpha blending dose not work right as you would need to sort the the instances list
/// Adds the necessary ECS resources and render logic to enable rendering entities using the given [`SpecializedMaterial`]
/// asset type (which includes [`Material`] types).
pub struct ModelInstanceMaterialPlugin<M: SpecializedMaterial>(PhantomData<M>);

impl<M: SpecializedMaterial> Default for ModelInstanceMaterialPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: SpecializedMaterial> Plugin for ModelInstanceMaterialPlugin<M> {
    fn build(&self, app: &mut App) {
        app.add_asset::<M>()
            .add_plugin(ExtractComponentPlugin::<Handle<M>>::default())
            .add_plugin(RenderAssetPlugin::<M>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent3d, DrawMaterial<M>>()
                .add_render_command::<Opaque3d, DrawMaterial<M>>()
                .add_render_command::<AlphaMask3d, DrawMaterial<M>>()
                .init_resource::<ModelDrawMaterialPipeline<M>>() // <-----------------
                .init_resource::<SpecializedPipelines<ModelDrawMaterialPipeline<M>>>() // <-----------------
                .add_system_to_stage(RenderStage::Queue, queue_material_meshes::<M>);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_material_meshes<M: SpecializedMaterial>(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    material_pipeline: Res<ModelDrawMaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedPipelines<ModelDrawMaterialPipeline<M>>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    msaa: Res<Msaa>,
    shared_mesh: Res<RenderAssets<SharedMesh>>,
    render_materials: Res<RenderAssets<M>>,
    material_meshes: Query<(Entity, &Handle<SharedMesh>, &Handle<M>, &MeshUniform)>,
    mut views: Query<(
        //Entity,
        //&ExtractedView,
        //&VisibleEntities,
        &mut RenderPhase<Opaque3d>,
        &mut RenderPhase<AlphaMask3d>,
        &mut RenderPhase<Transparent3d>,
    )>,
) {
    for (mut opaque_phase, mut alpha_mask_phase, mut transparent_phase) in views.iter_mut() {
        let draw_opaque_pbr = opaque_draw_functions
            .read()
            .get_id::<DrawMaterial<M>>()
            .unwrap();
        let draw_alpha_mask_pbr = alpha_mask_draw_functions
            .read()
            .get_id::<DrawMaterial<M>>()
            .unwrap();
        let draw_transparent_pbr = transparent_draw_functions
            .read()
            .get_id::<DrawMaterial<M>>()
            .unwrap();

        //let inverse_view_matrix = view.transform.compute_matrix().inverse();
        //let inverse_view_row_2 = inverse_view_matrix.row(2);
        let mesh_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

        //for visible_entity in &visible_entities.entities {
        for (entity, mesh_handel, material_handle, _mesh_uniform) in material_meshes.iter() {
            let shard_mesh = shared_mesh.get(mesh_handel).unwrap();
            if let Some(material) = render_materials.get(material_handle) {
                let mut mesh_key = mesh_key;

                mesh_key |= MeshPipelineKey::from_primitive_topology(shard_mesh.primitive_topology);

                let alpha_mode = M::alpha_mode(material);
                if let AlphaMode::Blend = alpha_mode {
                    mesh_key |= MeshPipelineKey::TRANSPARENT_MAIN_PASS
                }

                let specialized_key = M::key(material);
                let pipeline_id = pipelines.specialize(
                    &mut pipeline_cache,
                    &material_pipeline,
                    (mesh_key, specialized_key),
                );

                // NOTE: row 2 of the inverse view matrix dotted with column 3 of the model matrix
                // gives the z component of translation of the mesh in view space
                let mesh_z = 0.0; //inverse_view_row_2.dot(mesh_uniform.transform.col(3));
                match alpha_mode {
                    AlphaMode::Opaque => {
                        opaque_phase.add(Opaque3d {
                            entity: entity,
                            draw_function: draw_opaque_pbr,
                            pipeline: pipeline_id,
                            // NOTE: Front-to-back ordering for opaque with ascending sort means near should have the
                            // lowest sort key and getting further away should increase. As we have
                            // -z in front of the camera, values in view space decrease away from the
                            // camera. Flipping the sign of mesh_z results in the correct front-to-back ordering
                            distance: -mesh_z,
                        });
                    }
                    AlphaMode::Mask(_) => {
                        alpha_mask_phase.add(AlphaMask3d {
                            entity: entity,
                            draw_function: draw_alpha_mask_pbr,
                            pipeline: pipeline_id,
                            // NOTE: Front-to-back ordering for alpha mask with ascending sort means near should have the
                            // lowest sort key and getting further away should increase. As we have
                            // -z in front of the camera, values in view space decrease away from the
                            // camera. Flipping the sign of mesh_z results in the correct front-to-back ordering
                            distance: -mesh_z,
                        });
                    }
                    AlphaMode::Blend => {
                        transparent_phase.add(Transparent3d {
                            entity: entity,
                            draw_function: draw_transparent_pbr,
                            pipeline: pipeline_id,
                            // NOTE: Back-to-front ordering for transparent with ascending sort means far should have the
                            // lowest sort key and getting closer should increase. As we have
                            // -z in front of the camera, the largest distance is -far with values increasing toward the
                            // camera. As such we can just use mesh_z as the distance
                            distance: mesh_z,
                        });
                    }
                }
            }
        }
        //}
    }
}

pub struct ModelDrawMaterialPipeline<M: SpecializedMaterial> {
    pub mesh_pipeline: MeshPipeline,
    pub material_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    marker: PhantomData<M>,
}

impl<M: SpecializedMaterial> SpecializedPipeline for ModelDrawMaterialPipeline<M> {
    type Key = (MeshPipelineKey, M::Key);

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut descriptor = self.mesh_pipeline.specialize(key.0);

        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }
        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.material_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
        ]);

        M::specialize(key.1, &mut descriptor);
        descriptor
    }
}

impl<M: SpecializedMaterial> FromWorld for ModelDrawMaterialPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let material_layout = M::bind_group_layout(render_device);

        ModelDrawMaterialPipeline {
            mesh_pipeline: world.get_resource::<MeshPipeline>().unwrap().clone(),
            material_layout,
            vertex_shader: M::vertex_shader(asset_server),
            fragment_shader: M::fragment_shader(asset_server),
            marker: PhantomData,
        }
    }
}

type DrawMaterial<M> = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<M, 1>,
    SetMeshBindGroup<2>,
    DrawMeshInstanced,
);

pub struct SetMaterialBindGroup<M: SpecializedMaterial, const I: usize>(PhantomData<M>);
impl<M: SpecializedMaterial, const I: usize> EntityRenderCommand for SetMaterialBindGroup<M, I> {
    type Param = (SRes<RenderAssets<M>>, SQuery<Read<Handle<M>>>);
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (materials, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material_handle = query.get(item).unwrap();
        let material = materials.into_inner().get(material_handle).unwrap();
        pass.set_bind_group(
            I,
            M::bind_group(material),
            M::dynamic_uniform_indices(material),
        );
        RenderCommandResult::Success
    }
}

pub struct DrawMeshInstanced;
impl EntityRenderCommand for DrawMeshInstanced {
    type Param = (
        SRes<RenderAssets<SharedMesh>>,
        SQuery<Read<Handle<SharedMesh>>>,
        SQuery<Read<DrawIndexedIndirectList>>,
        SQuery<Read<InstanceBuffer>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (shared_mesh, h_mesh, draw_indirect_query, instance_buffer_query): SystemParamItem<
            'w,
            '_,
            Self::Param,
        >,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_handel = h_mesh.get(item).unwrap();
        let shard_mesh = shared_mesh.into_inner().get(mesh_handel).unwrap();
        let instance_buffer = instance_buffer_query.get(item).unwrap();
        let draw_indirect_list = draw_indirect_query.get(item).unwrap();

        pass.set_vertex_buffer(0, shard_mesh.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));
        pass.set_index_buffer(shard_mesh.index_buffer.slice(..), 0, IndexFormat::Uint32);

        // by setting the vertex, index buffer and instance once you can call draw_indexed multiple times and offset what to use within the buffers
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

// can work on doing latter, on the user side there is no different
// get all material and mesh combos as keys and merge all instances together to reduce number of gpu state changes
// create a new entity with the key and value that will be rendered
fn merge_entitys<M: SpecializedMaterial>(
    query: Query<(Entity, &Handle<M>, &Handle<SharedMesh>, &ModelInstanceList)>,

    mut com: Commands,
) {
    let mut mat_map: HashMap<(&Handle<M>, &Handle<SharedMesh>), ModelInstanceList> = HashMap::new();
    for (e, mat, shared_mesh, inst_list) in query.iter() {
        let k = (mat, shared_mesh);
        if mat_map.contains_key(&k) {
            let tk = mat_map.get_mut(&k).unwrap();
            tk.instance_list
                .append(&mut inst_list.instance_list.clone());
        } else {
            mat_map.insert(k, inst_list.clone());
        }
    }
}
