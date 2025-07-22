import package::{
    camera_shader::camera,
    light_shader::{light, light_main},
    fog_shader::{fog, fog_main, FogValue}
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) screen_t: f32,
    @location(4) view_depth: f32,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );
    
    var out: VertexOutput;
    out.world_position = (model_matrix * vec4<f32>(model.position, 1.0)).xyz;
    out.clip_position = camera.view_proj * vec4<f32>(out.world_position, 1.0);
    out.world_normal = normalize(normal_matrix * model.normal);
    out.color = model.color;
    
    let ndc_pos = out.clip_position.xy / out.clip_position.w;
    out.screen_t = (ndc_pos.y + 1.0) * 0.5;
    
    out.view_depth = distance(out.world_position, camera.view_pos.xyz);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let fog_value = fog_main(in.screen_t, in.view_depth);
    
    let object_color = light_main(in.world_position, in.world_normal) * in.color;
    let fogged_color = object_color * (1.0 - fog_value.factor) + fog_value.color.rgb * fog_value.factor;
    return vec4<f32>(fogged_color, 1.0);
}