#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct


//[[block]]
struct LineMaterial {
    color: vec4<f32>;
};
[[group(1), binding(0)]]
var<uniform> material: LineMaterial;


[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

// in instance
struct InstanceInput {
    // transfrom
    [[location(5)]] model_matrix_0: vec4<f32>;
    [[location(6)]] model_matrix_1: vec4<f32>;
    [[location(7)]] model_matrix_2: vec4<f32>;
    [[location(8)]] model_matrix_3: vec4<f32>;
    // other
    [[location(9)]] color: vec4<f32>;
};
fn transform_from_instance(instance: InstanceInput) -> mat4x4<f32> {
  return mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
}
// in vertex
struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] color: vec4<f32>;
    //[[location(1)]] normal: vec4<f32>;
    //[[location(3)]] uv_i: vec4<u32>;
    
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;

  
    [[location(0)]] color: vec4<f32>;
    
};


[[stage(vertex)]]
fn vertex(vertex: Vertex, instance: InstanceInput) -> VertexOutput {

    let world_matrix = mesh.model * transform_from_instance(instance); 

    let position =  vertex.position;
    let world_position =  world_matrix * vec4<f32>(position, 1.0);

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
    out.color = instance.color * vertex.color;
    //out.normal = vertex.normal.xyz;
    
    //let uv = vec2<f32>(f32(vertex.uv_i.x) / 255.0,f32(vertex.uv_i.y)/255.0);
    //let index = (vertex.uv_i.b << u32(8)) | vertex.uv_i.a;
    //out.uv = uv;
    //out.index = index;

    // lighting // debug
    //let light_l = (dot(normalize( vec3<f32>(0.3,1.0,0.1)),out.normal) * 0.6) + 0.4;
    
    # ifdef DEBUG_UV 
    //    out.color = vec3<f32>( out.uv.x ,out.uv.y ,0.5);
    # endif

    # ifdef IS_LIGHTING
    //    out.color = out.color * light_l;
    # endif

    return out;
}

// COLOR_TEXTURE
//[[group(1), binding(1)]]
//var base_color_texture: texture_2d_array<f32>;
//[[group(1), binding(2)]]
//var base_color_sampler: sampler;

struct FragIn {
    [[location(0)]] color: vec4<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragIn) -> [[location(0)]] vec4<f32> {
let out = in.color;
    if (out.a <= 0.5) {
        discard;
    }
    return out;
}
