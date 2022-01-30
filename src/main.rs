use std::path::Path;

use bevy::{
    asset::{AssetLoader, AssetPath, BoxedFuture, LoadContext, LoadedAsset},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::SpecializedMaterial,
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_resource::{
            std140, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
            BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages, Extent3d,
            PrimitiveTopology, RenderPipelineDescriptor, SamplerBindingType, ShaderStages,
            TextureDimension, TextureSampleType, TextureViewDimension,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::ImageType,
    },
};

use bevy_egui::{
    egui::{self},
    EguiContext, EguiPlugin,
};
use bevy_fly_camera::FlyCamera;
use serde::Deserialize;

//use chunk_pipeline::ChunkMesh;
use noise::{NoiseFn, OpenSimplex};
use rand::prelude::*;
use rendering::{
    instancing::{InstanceModelPlugin, InstanceRaw, ModelInstanceList},
    mesh::{ChunkMeshvertex, LineMeshvertex, SharedMesh, SharedMeshPlugin},
    model_draw_pipeline::{ModelDrawMaterialPipeline, ModelInstanceMaterialPlugin},
};

use voxel::voxel::{Chunk, ChunkKey, VoxelMap};

use crate::rendering::instancing::Instance;

mod rendering;
mod voxel;

// world size in chunks
const WORLD_SIZE_XZ: i32 = 6;
const WORLD_SIZE_Y: i32 = 9;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(WindowDescriptor {
            // uncomment for unthrottled FPS
            //  vsync: false,
            ..Default::default()
        })
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(TestPlugin)
        .run();
}

struct TestPlugin;

impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        // ------ custom render setup
        app.add_plugin(SharedMeshPlugin);
        app.add_plugin(InstanceModelPlugin);
        // material needs both shared mesh and instancing
        app.add_plugin(ModelInstanceMaterialPlugin::<ChunkOpaqueMaterial>::default());
        app.add_plugin(ModelInstanceMaterialPlugin::<LineMaterial>::default());

        //
        app.add_asset::<ImageArray>()
            .init_asset_loader::<CustomAssetLoader>()
            .add_startup_system(set_up_scene)
            .add_system(consume_image_array)
            .add_plugin(bevy_fly_camera::FlyCameraPlugin)
            .add_plugin(EguiPlugin)
            .add_system(ui_info);
    }
}

fn ui_info(
    egui_context: ResMut<EguiContext>,
    time: ResMut<Time>,
    q: Query<(&Handle<SharedMesh>, &ModelInstanceList)>,
    shard_meshes: ResMut<Assets<SharedMesh>>,
    //mut debug_materials: ResMut<Assets<LineMaterial>>,
) {
    let dt = time.delta_seconds();
    let mut num_sub_meshs = 0;
    let mut num_of_objects_to_draw = 0;
    for (h_mesh, draw_list) in q.iter() {
        let mesh = shard_meshes.get(h_mesh).unwrap();
        num_sub_meshs += mesh.mesh_handel.len();
        num_of_objects_to_draw += draw_list.instance_list.len();
    }
    egui::Window::new("info").show(egui_context.ctx(), |ui| {
        ui.label(format!("fps: {:?}", 1.0 / dt));
        ui.label(format!("sub_meshes: {:?}", num_sub_meshs));
        ui.label(format!("objects_to_draw: {:?}", num_of_objects_to_draw));
    });
}

//-----------------------------

#[derive(Component)]
pub struct VolumeMap {
    pub val: VoxelMap,
}

fn set_up_scene(
    mut com: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut shard_meshes: ResMut<Assets<SharedMesh>>,
    //mut materials: ResMut<Assets<NewMaterial>>,
    mut debug_materials: ResMut<Assets<LineMaterial>>,
    //mut images: ResMut<Assets<Image>>,
    //  mut cons: ResMut<Assets<ImageAssetConfig>>,
    asset_server: ResMut<AssetServer>,
    queue: Res<RenderQueue>,
    device: Res<RenderDevice>,
) {
    let mut volm = VolumeMap {
        val: VoxelMap::new((16, 16, 16)),
    };

    let mut chunk_shared_mesh = SharedMesh::new::<ChunkMeshvertex>(
        "test_mesh_s".into(),
        &device,
        (14_000 * 4028, 20_000 * 4028), // buffer_size (vertex, index)
        PrimitiveTopology::TriangleList,
    );
    let mut draw_list = ModelInstanceList {
        instance_list: vec![],
    };

    let mut debug_mesh = SharedMesh::new::<LineMeshvertex>(
        "test_mesh_s".into(),
        &device,
        (1 * 4028, 1 * 4028),
        PrimitiveTopology::LineList,
    );
    let mut debug_draw_list = ModelInstanceList {
        instance_list: vec![],
    };

    let mut l_v = vec![];
    let mut l_i = vec![];
    // gen box mesh
    {
        l_v.push(LineMeshvertex::new([0.0, 0.0, 0.0], [1.0, 0.7, 0.0, 1.0])); // 0
        l_v.push(LineMeshvertex::new([16.0, 0.0, 0.0], [1.0, 0.7, 0.0, 1.0])); // 1
        l_v.push(LineMeshvertex::new([16.0, 0.0, 16.0], [1.0, 0.7, 0.0, 1.0])); // 2
        l_v.push(LineMeshvertex::new([0.0, 0.0, 16.0], [1.0, 0.7, 0.0, 1.0])); // 3

        l_v.push(LineMeshvertex::new([0.0, 16.0, 0.0], [1.0, 0.7, 0.0, 1.0])); // 4
        l_v.push(LineMeshvertex::new([16.0, 16.0, 0.0], [1.0, 0.7, 0.0, 1.0])); // 5
        l_v.push(LineMeshvertex::new(
            [16.0, 16.0, 16.0],
            [1.0, 0.7, 0.0, 1.0],
        )); // 6
        l_v.push(LineMeshvertex::new([0.0, 16.0, 16.0], [1.0, 0.7, 0.0, 1.0])); // 7

        // bot
        l_i.push(0);
        l_i.push(1);

        l_i.push(1);
        l_i.push(2);

        l_i.push(2);
        l_i.push(3);

        l_i.push(3);
        l_i.push(0);

        // top

        l_i.push(4);
        l_i.push(5);

        l_i.push(5);
        l_i.push(6);

        l_i.push(6);
        l_i.push(7);

        l_i.push(7);
        l_i.push(4);
        // vertical

        l_i.push(0);
        l_i.push(4);

        l_i.push(1);
        l_i.push(5);

        l_i.push(2);
        l_i.push(6);

        l_i.push(3);
        l_i.push(7);
    }

    let debug_box_mesh = debug_mesh.get_handel(&l_v, &l_i, &queue);

    // ---------------------------- chunk gen range
    // world size in chunks
    let rr = WORLD_SIZE_XZ; // x and z size
    let ry = WORLD_SIZE_Y; // y size
    for x in -rr..=rr {
        for y in -ry..=ry {
            for z in -rr..=rr {
                let mut ch = Chunk::new(16);
                volm.val.add_chunk(x, y, z, ch);
            }
        }
    }

    let p_n = noise::SuperSimplex::new();
    let s_n = OpenSimplex::new();
    let scl = 0.0731;
    let scl_2 = 0.00613;
    let scl_3 = 0.0313;

    let mut r_n = rand::thread_rng();

    // world gen
    let crr = rr * 32;
    let cry = ry * 32;
    for x in -crr..crr {
        println!("w_gen x::{}", x);

        for z in -crr..crr {
            let s = (s_n.get([x as f64 * scl_2, z as f64 * scl_2]) * 10.0).exp();
            let mut s2 = s_n.get([x as f64 * scl_3, z as f64 * scl_3]) * 8.0;

            for y in -cry..cry {
                let mut o = 0;

                let d_n = voxel::voxel::noise_3d(x as f32 * 0.2, 0.0, z as f32 * 0.09) as f64;
                if d_n >= 0.0 {
                    s2 = s2.abs()
                }
                if (s + s2) >= (y as f64) {
                    o = r_n.gen_range(14..=15); // <--- grass layer
                    if (s + s2) >= (y as f64) + ((d_n * 2.0) + 1.0).abs() {
                        o = r_n.gen_range(1..=5); // dirt
                    }
                    let p =
                        p_n.get([x as f64 * scl, y as f64 * scl, z as f64 * scl]) * (32.0 * d_n);
                    if (p) <= -(y as f64 + 20.3).clamp(0.6, 20.0) {
                        o = 0;
                    }
                }
                if y as f32 <= ((d_n * 6.0) - 10.0) as f32 && o != 0 {
                    if r_n.gen_ratio(1, 38) {
                        o = r_n.gen_range(11..=13); // <-- ore
                    } else {
                        o = r_n.gen_range(6..=8); // <-- stone
                    }

                    if y as f32 <= ((d_n * 26.0) - 100.0) as f32 && o != 0 {
                        if r_n.gen_ratio(4, 36) {
                            o = r_n.gen_range(11..=13); // <-- ore
                        } else {
                            o = r_n.gen_range(9..=10); // <-- cobble stone
                        }
                    }
                }

                volm.val.set_voxel(x, y, z, o);
            }
        }
    }

    // chunk meshing
    for x in -rr..=rr {
        println!("mesh x::{}", x);
        chunk_shared_mesh.print_size();
        for y in -ry..=ry {
            for z in -rr..=rr {
                let mut c_verts = Vec::new();
                let mut c_index = Vec::new();
                let mut s_i = 0;
                volm.val.update_chunk_mesh(
                    ChunkKey::new((x, y, z)),
                    &mut c_verts,
                    &mut c_index,
                    &mut s_i,
                    false,
                );

                if c_verts.is_empty() {
                    continue;
                }
                let mut n_cmv = vec![];
                for tv in c_verts.iter() {
                    n_cmv.push(ChunkMeshvertex::new(
                        tv.position,
                        tv.normal,
                        tv.color,
                        tv.uv_0,
                        tv.index,
                    ))
                }

                let inst = Instance {
                    position: Vec3::new(x as f32 * 16.0, y as f32 * 16.0, z as f32 * 16.0),
                    scale: Vec3::new(1., 1., 1.),
                    color: Color::WHITE.into(),
                    rotation: Quat::from_axis_angle(Vec3::new(0., 0., 0.), 0.0),
                };
                let sub_mesh_handel = chunk_shared_mesh.get_handel(&n_cmv, &c_index, &queue);
                draw_list
                    .instance_list
                    .push(rendering::instancing::ModelInstance {
                        mesh: sub_mesh_handel,
                        instance: inst.clone(),
                        inst_index: 0,
                    });

                debug_draw_list
                    .instance_list
                    .push(rendering::instancing::ModelInstance {
                        mesh: debug_box_mesh.clone(),
                        instance: inst.clone(),
                        inst_index: 0,
                    });
            }
        }
    }

    chunk_shared_mesh.print_size();

    // ------------------------------------------------------------------------------------------------------

    let img_array: Handle<ImageArray> = asset_server.load("data/block_texture_config.ron");

    // instance render
    com.spawn().insert_bundle((
        img_array,
        shard_meshes.add(chunk_shared_mesh),
        draw_list,
        Transform::from_rotation(Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 0.0)),
        GlobalTransform::default(),
        Visibility::default(),
        ComputedVisibility::default(),
    ));

    // debug bonding box
    com.spawn().insert_bundle((
        shard_meshes.add(debug_mesh),
        debug_draw_list,
        debug_materials.add(LineMaterial {
            color: Color::WHITE.into(),
            b_draw: false,
        }),
        Transform::from_rotation(Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 0.0)),
        GlobalTransform::default(),
        Visibility::default(),
        ComputedVisibility::default(),
    ));

    // ------- test obj
    com.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..Default::default()
    });
    com.spawn_bundle(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_axis_angle(Vec3::new(1.0, 0.1, 0.0), 30.0)),
        directional_light: DirectionalLight {
            ..Default::default()
        },
        ..Default::default()
    });

    com.spawn()
        .insert_bundle(PerspectiveCameraBundle::new_3d())
        .insert(FlyCamera::default());
}

// extract image array out to material
fn consume_image_array(
    q: Query<(Entity, &Handle<ImageArray>), Without<Handle<ChunkOpaqueMaterial>>>,
    mut imagearrays: ResMut<Assets<ImageArray>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ChunkOpaqueMaterial>>,

    mut com: Commands,
) {
    for (e, ih) in q.iter() {
        match imagearrays.get_mut(ih) {
            Some(img) => {
                //img.img.reinterpret_stacked_2d_as_array(16);
                let h = images.add(img.img.clone());
                com.get_or_spawn(e)
                    .insert(materials.add(ChunkOpaqueMaterial {
                        color: Color::WHITE.into(),
                        base_color_texture: Some(h.clone()),
                    }))
                    .remove::<Handle<ImageArray>>();
            }
            None => {}
        }
    }
}

// image asset loading
//---------------------------------------------------------------

#[derive(Debug, TypeUuid)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
pub struct ImageArray {
    pub img: Image,
    pub config: ImageArrayConfig,
}

#[derive(Debug, Deserialize, TypeUuid, Clone)]
#[uuid = "77cadc56-aa9c-4543-8640-a018b74b5052"]
pub struct ImageArrayConfig {
    pub pixel_size: u32,
    pub layer_depth: u32,
    pub paths_id: Vec<(String, u32)>,
}

#[derive(Default)]
pub struct CustomAssetLoader;

impl AssetLoader for CustomAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let config = ron::de::from_bytes::<ImageArrayConfig>(bytes)?;
            // ------------------------------------------------

            let mut image_list = vec![];

            for (c_path, c_id) in config.paths_id.iter() {
                let path = Path::new(c_path);
                let raw = load_context.read_asset_bytes(path).await.unwrap();
                image_list.push(Image::from_buffer(&raw, ImageType::Extension("png")).unwrap());
            }

            let mut data = vec![];
            for i in image_list.iter() {
                data.append(&mut i.data.clone())
            }

            let format = image_list[0].texture_descriptor.format;
            let out_image = Image::new(
                Extent3d {
                    width: config.pixel_size,
                    height: config.pixel_size,
                    depth_or_array_layers: config.layer_depth,
                },
                TextureDimension::D2,
                data,
                format,
            );

            // --------------------------------------------------
            // image path
            let path = Path::new(&config.paths_id[0].0);

            let asset_path = AssetPath::new_ref(path, None);
            let asset = LoadedAsset::new(ImageArray {
                img: out_image,
                config: config.clone(),
            })
            .with_dependency(asset_path); // <- Mark it as a dependency so the server knows to load it

            load_context.set_default_asset(asset);
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

// defining materials
// ---------------------------------------------------------------

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "4ee9c363-1124-4113-890e-199d81b00281"]
pub struct ChunkOpaqueMaterial {
    pub color: Color,
    pub base_color_texture: Option<Handle<Image>>,
}

#[derive(Clone)]
pub struct GpuNewMaterial {
    _buffer: Buffer,
    pub bind_group: BindGroup,
    pub base_color_texture: Option<Handle<Image>>,
}

impl RenderAsset for ChunkOpaqueMaterial {
    type ExtractedAsset = ChunkOpaqueMaterial;
    type PreparedAsset = GpuNewMaterial;

    type Param = (
        SRes<RenderDevice>,
        SRes<ModelDrawMaterialPipeline<Self>>, // <- pipeline
        SRes<RenderAssets<Image>>,
    );
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, custom_pipeline, gpu_image): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let (base_color_texture_view, base_color_sampler) = if let Some(result) = custom_pipeline
            .mesh_pipeline
            .get_image_texture(gpu_image, &extracted_asset.base_color_texture)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(extracted_asset));
        };

        let color: Vec4 = extracted_asset.color.as_rgba_linear().into();
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: std140::Std140::as_bytes(&std140::AsStd140::as_std140(&color)),
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(base_color_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(base_color_sampler),
                },
            ],
            label: None,
            layout: &custom_pipeline.material_layout,
        });

        Ok(GpuNewMaterial {
            _buffer: buffer,
            bind_group,
            base_color_texture: extracted_asset.base_color_texture,
        })
    }
}

impl SpecializedMaterial for ChunkOpaqueMaterial {
    type Key = ();

    fn key(_: &<ChunkOpaqueMaterial as RenderAsset>::PreparedAsset) -> Self::Key {}

    fn specialize(_: Self::Key, descriptor: &mut RenderPipelineDescriptor) {
        descriptor.vertex.buffers = vec![ChunkMeshvertex::desc()];
        descriptor.vertex.buffers.push(InstanceRaw::desc());

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

        descriptor.vertex.entry_point = "vertex".into();
        descriptor.fragment.as_mut().unwrap().entry_point = "fragment".into();
    }

    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        asset_server.watch_for_changes().unwrap();
        Some(asset_server.load("shaders/instancing.wgsl"))
    }

    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        asset_server.watch_for_changes().unwrap();
        Some(asset_server.load("shaders/instancing.wgsl"))
    }

    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
        })
    }
    fn alpha_mode(material: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        AlphaMode::Opaque
    }
}

// -----------------------------------------------------------------
// debug line mat

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "4ee9c363-1124-7777-890e-199d81b00281"]
pub struct LineMaterial {
    pub color: Color,
    pub b_draw: bool,
}

#[derive(Clone)]
pub struct GpuLineMaterial {
    _buffer: Buffer,
    pub bind_group: BindGroup,
}

impl RenderAsset for LineMaterial {
    type ExtractedAsset = LineMaterial;
    type PreparedAsset = GpuLineMaterial;

    type Param = (
        SRes<RenderDevice>,
        SRes<ModelDrawMaterialPipeline<Self>>, // <- pipeline
    );
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, custom_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let color: Vec4 = extracted_asset.color.as_rgba_linear().into();
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: std140::Std140::as_bytes(&std140::AsStd140::as_std140(&color)),
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
            layout: &custom_pipeline.material_layout,
        });

        Ok(GpuLineMaterial {
            _buffer: buffer,
            bind_group,
        })
    }
}

impl SpecializedMaterial for LineMaterial {
    type Key = ();

    fn key(_: &<LineMaterial as RenderAsset>::PreparedAsset) -> Self::Key {}

    fn specialize(_: Self::Key, descriptor: &mut RenderPipelineDescriptor) {
        descriptor.vertex.buffers = vec![LineMeshvertex::desc()];
        descriptor.vertex.buffers.push(InstanceRaw::desc());

        descriptor.primitive.cull_mode = None;

        let mut shader_defs = vec![];

        descriptor.vertex.shader_defs.append(&mut shader_defs);

        if let Some(frag) = &mut descriptor.fragment {
            frag.shader_defs.append(&mut shader_defs);
        }

        descriptor.vertex.entry_point = "vertex".into();
        descriptor.fragment.as_mut().unwrap().entry_point = "fragment".into();
    }

    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        asset_server.watch_for_changes().unwrap();
        Some(asset_server.load("shaders/line_instancing.wgsl"))
    }

    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        asset_server.watch_for_changes().unwrap();
        Some(asset_server.load("shaders/line_instancing.wgsl"))
    }

    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
            ],
            label: None,
        })
    }
    fn alpha_mode(material: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        AlphaMode::Blend
    }
}
