struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0) var<uniform> camera: CameraUniform;

struct LightUniform {
    position: vec3<f32>,
    color: vec3<f32>,
}
@group(2) @binding(0) var<uniform> light: LightUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
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
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) world_normal: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2
    );
    
    var out: VertexOutput;
    out.world_position = (model_matrix * vec4<f32>(model.position, 1.0)).xyz;
    out.clip_position = camera.view_proj * vec4<f32>(out.world_position, 1.0);
    out.tex_coords = vec2<f32>(model.tex_coords.x, 1.0 - model.tex_coords.y); // Инвертируем V-координату
    out.world_normal = normalize(normal_matrix * model.normal);
    return out;
}

@group(0) @binding(0) var tex_sampler: sampler;
@group(0) @binding(1) var tex: texture_2d<f32>;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let ambient_strength = 0.1;
    let ambient = ambient_strength * light.color;
    
    let light_dir = normalize(light.position - input.world_position);
    let diffuse_strength = max(dot(input.world_normal, light_dir), 0.0);
    let diffuse = diffuse_strength * light.color;
    
    let view_dir = normalize(camera.view_pos.xyz - input.world_position);
    let reflect_dir = reflect(-light_dir, input.world_normal);
    let specular_strength = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
    let specular = specular_strength * light.color;
    
    let base_color = textureSample(tex, tex_sampler, input.tex_coords);
    let result = (ambient + diffuse + specular) * base_color.rgb;
    return vec4<f32>(result, base_color.a);
}