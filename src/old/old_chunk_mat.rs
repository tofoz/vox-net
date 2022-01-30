use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::{AlphaMode, MaterialPipeline, SpecializedMaterial},
    prelude::{AssetServer, Color, Handle, Image, Shader},
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_resource::{
            std140::{AsStd140, Std140},
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
            BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages,
            RenderPipelineDescriptor, SamplerBindingType, ShaderStages, TextureSampleType,
            TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat,
            VertexStepMode,
        },
        renderer::RenderDevice,
    },
};
use prism_math::Vec4;

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "4ee9c363-1124-4113-890e-199d81b00281"]
pub struct CustomMaterial {
    pub color: Color,
    pub base_color_texture: Option<Handle<Image>>,
    pub double_sided: bool,
    pub unlit: bool,
    pub alpha_mode: AlphaMode,
}

impl Default for CustomMaterial {
    fn default() -> Self {
        Self {
            color: Color::rgb(1.0, 1.0, 1.0),
            base_color_texture: None,

            double_sided: false,
            unlit: false,
            alpha_mode: AlphaMode::Opaque,
        }
    }
}

impl From<Handle<Image>> for CustomMaterial {
    fn from(texture: Handle<Image>) -> Self {
        Self {
            base_color_texture: Some(texture),
            ..Default::default()
        }
    }
}

#[derive(Clone)]
pub struct GpuCustomMaterial {
    _buffer: Buffer,
    bind_group: BindGroup,

    pub base_color_texture: Option<Handle<Image>>,

    pub alpha_mode: AlphaMode,
}

impl RenderAsset for CustomMaterial {
    type ExtractedAsset = CustomMaterial;
    type PreparedAsset = GpuCustomMaterial;

    type Param = (
        SRes<RenderDevice>,
        SRes<MaterialPipeline<CustomMaterial>>, // <- pipeline
        SRes<RenderAssets<Image>>,
    );
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, material_pipeline, gpu_image): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let (base_color_texture_view, base_color_sampler) = if let Some(result) = material_pipeline
            .mesh_pipeline
            .get_image_texture(gpu_image, &extracted_asset.base_color_texture)
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(extracted_asset));
        };
        let color = Vec4::from_slice(&extracted_asset.color.as_linear_rgba_f32());
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: color.as_std140().as_bytes(),
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
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuCustomMaterial {
            _buffer: buffer,
            bind_group,
            base_color_texture: extracted_asset.base_color_texture,
            alpha_mode: extracted_asset.alpha_mode,
        })
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct CKey {}

impl SpecializedMaterial for CustomMaterial {
    type Key = CKey;

    fn key(material: &<Self as RenderAsset>::PreparedAsset) -> Self::Key {
        CKey {}
    }

    fn specialize(key: Self::Key, descriptor: &mut RenderPipelineDescriptor) {
        // let mut descriptor = self.pipeline.specialize(key);

        // !! attributes are sorted alphabetically
        let vertex_attributes = vec![
            VertexAttribute {
                // uv
                format: VertexFormat::Uint8x4,
                offset: 8 + 12,
                shader_location: 1,
            },
            // position
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 8,
                shader_location: 0,
            },
            // normal
            VertexAttribute {
                format: VertexFormat::Unorm8x4,
                offset: 4,
                shader_location: 3,
            },
            // color
            VertexAttribute {
                format: VertexFormat::Unorm8x4,
                offset: 0,
                shader_location: 2,
            },
        ];

        let vertex_array_stride = 4 + 4 + 4 + (4 * 3);

        let mut shader_defs = vec![];
        if false {
            shader_defs.push("DEBUG_UV".to_string());
        }
        if true {
            shader_defs.push("IS_LIGHTING".to_string());
        }

        descriptor.vertex.shader_defs.append(&mut shader_defs); // vertex
        if let Some(frag) = &mut descriptor.fragment {
            frag.shader_defs.append(&mut shader_defs); // frag
        }
        descriptor.vertex.buffers = vec![VertexBufferLayout {
            array_stride: vertex_array_stride,
            step_mode: VertexStepMode::Vertex,
            attributes: vertex_attributes,
        }];
    }

    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(asset_server.load("shaders/custom_material.wgsl"))
    }
    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(asset_server.load("shaders/custom_material.wgsl"))
    }

    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(Vec4::std140_size_static() as u64),
                    },
                    count: None,
                }, // Base Color Texture
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
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
        material.alpha_mode
    }

    fn dynamic_uniform_indices(material: &<Self as RenderAsset>::PreparedAsset) -> &[u32] {
        &[]
    }
}
