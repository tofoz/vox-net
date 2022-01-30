struct View {
    view_proj: mat4x4<f32>;
    inverse_view: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
    near: f32;
    far: f32;
    width: f32;
    height: f32;
};


struct Mesh {
    model: mat4x4<f32>;
    inverse_transpose_model: mat4x4<f32>;
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32;
};

let MESH_FLAGS_SHADOW_RECEIVER_BIT: u32 = 1u;



struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] uv_i: vec4<u32>;  
    [[location(2)]] color: vec4<f32>;
    [[location(3)]] normal: vec3<f32>;
};

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

[[group(0), binding(0)]]
var<uniform> view: View;

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] uv: vec2<f32>; 
    [[location(1)]] index: u32;
    [[location(2)]] color: vec3<f32>;
    [[location(3)]] normal: vec3<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position + vec3<f32>(7.0,0.0,0.0), 1.0) ;

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;

    let uv = vec2<f32>(f32(vertex.uv_i.x)/255.0,f32(vertex.uv_i.y)/255.0);
    let index = (vertex.uv_i.b << u32(8)) | vertex.uv_i.a;
    out.uv = uv;
    out.index = index;

    // Project the world position of the mesh into screen position
    out.clip_position = view.view_proj * mesh.model * vec4<f32>(vertex.position, 1.0);
    out.normal = vertex.normal;
    out.color = vertex.color.rgb;

    if (index == u32(0)) {
        out.color = vec3<f32>(0.21,0.8,0.34);
    }
    if (index == u32(1)) {
        out.color = vec3<f32>(1.21,0.8,0.35) * 0.25;
    }
    if (index == u32(2)) {
        out.color = vec3<f32>(0.1,0.2,0.81);
    }


// lighting // debug
    let light_l = (dot(normalize( vec3<f32>(0.3,1.0,0.1)),out.normal) * 0.6) + 0.4;
    
    # ifdef DEBUG_UV 
        out.color = vec3<f32>( out.uv.x ,out.uv.y ,0.5);
    # endif

    # ifdef IS_LIGHTING
        out.color = out.color * light_l;
    # endif

    return out;
}


struct FragIn {
    [[location(0)]] uv: vec2<f32>;
    [[location(1)]] index: u32;
    [[location(2)]] color: vec3<f32>;
    [[location(3)]] normal: vec3<f32>;
};

struct CustomMaterial {
    color: vec4<f32>;
    };
[[group(1), binding(0)]]
var<uniform> material: CustomMaterial;

[[group(1), binding(1)]]
var base_color_texture: texture_2d<f32>;
[[group(1), binding(2)]]
var base_color_sampler: sampler;

[[stage(fragment)]]
fn fragment(in: FragIn) -> [[location(0)]] vec4<f32> {
    return textureSample(base_color_texture, base_color_sampler, in.uv) * vec4<f32>( in.color,1.0);
}
