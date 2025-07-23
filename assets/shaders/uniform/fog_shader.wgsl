// fog_shader.wgsl
struct Fog {
    lower_color: vec4<f32>,
    upper_color: vec4<f32>,
    density: f32,
    start: f32,
};

@group(0) @binding(2) var<uniform> fog: Fog;

struct FogValue {
    color: vec4<f32>,
    factor: f32,
}

fn fog_main(screen_t: f32, depth: f32) -> FogValue {
    let fog_color = mix(fog.lower_color, fog.upper_color, screen_t);
    
    let adjusted_depth = max(0.0, depth - fog.start);
    let density_factor = fog.density * adjusted_depth;
    let factor = 1.0 - exp(-density_factor * density_factor * 0.5);
    
    return FogValue(fog_color, factor);
}