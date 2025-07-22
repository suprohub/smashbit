import package::{
    camera_shader::camera,
    fog_shader::{fog, fog_main}
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) screen_t: f32,
    @location(1) view_depth: f32,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array(
        vec2(-1.0, -1.0),
        vec2(3.0, -1.0),
        vec2(-1.0, 3.0)
    );
    
    let pos = positions[vertex_index];
    let clip_pos = vec4(pos, 0.0, 1.0);
    
    let screen_t = (pos.y + 1.0) * 0.5;
    
    return VertexOutput(
        clip_pos,
        screen_t,
        1000.0
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let fog_value = fog_main(in.screen_t, in.view_depth);
    return fog_value.color * fog_value.factor;
}