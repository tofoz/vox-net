use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_component::ExtractComponentPlugin,
        render_resource::{
            std140, BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer,
            BufferInitDescriptor, BufferUsages,
        },
        renderer::RenderDevice,
    },
};

use super::old_chunk_pipeline::CustomPipeline;

// todo: make the pipeline and material more generic

pub struct ChunkMaterialPlugin;

impl Plugin for ChunkMaterialPlugin {
    fn build(&self, app: &mut App) {
        // add a new material to pass into a custom render command
        app.add_asset::<MyMaterial>()
            .add_plugin(ExtractComponentPlugin::<Handle<MyMaterial>>::default())
            .add_plugin(RenderAssetPlugin::<MyMaterial>::default()); // <--------
    }
}

//   --------------------------------------------------------

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "4ee9c363-1124-4113-890e-199d81b00281"]
pub struct MyMaterial {
    pub color: Color,
    pub base_color_texture: Option<Handle<Image>>,
}

#[derive(Clone)]
pub struct GpuMyMaterial {
    _buffer: Buffer,
    pub bind_group: BindGroup,

    pub base_color_texture: Option<Handle<Image>>,
    //pub alpha_mode: AlphaMode,
}

impl RenderAsset for MyMaterial {
    type ExtractedAsset = MyMaterial;
    type PreparedAsset = GpuMyMaterial;

    type Param = (
        SRes<RenderDevice>,
        SRes<CustomPipeline>, // <- pipeline
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

        Ok(GpuMyMaterial {
            _buffer: buffer,
            bind_group,
            base_color_texture: extracted_asset.base_color_texture,
        })
    }
}
