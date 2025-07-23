
import package::uniform::camera_shader::camera;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}

@group(0) @binding(1) var<uniform> light: Light;

fn light_main(world_position: vec3<f32>, world_normal: vec3<f32>) -> vec3<f32> {
    // Ambient
    let ambient_strength = 0.1;
    let ambient = ambient_strength * light.color;
    
    // Diffuse
    let light_dir = normalize(light.position - world_position);
    let diffuse_strength = max(dot(world_normal, light_dir), 0.0);
    let diffuse = diffuse_strength * light.color;
    
    // Specular (Blinn-Phong)
    let view_dir = normalize(camera.view_pos.xyz - world_position);
    let halfway_dir = normalize(light_dir + view_dir);
    let specular_strength = pow(max(dot(world_normal, halfway_dir), 0.0), 32.0);
    let specular = specular_strength * light.color;

    return (ambient + diffuse + specular);
}

