use std::{collections::HashMap, fs, sync::Arc};

use crate::{
    camera_controller::CameraController,
    physics::Physics,
    renderer::{
        Renderer,
        pipeline::{InstanceRaw, color::ColoredVertex, texture::TexturedVertex},
        texture::Texture,
    },
};
use glam::{Mat3, Mat4, Vec2, Vec3};
use gltf::{Gltf, Node, Primitive};
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, sound::static_sound::StaticSoundData,
};
use rapier3d::{
    math::{Point, Vector},
    prelude::{ColliderBuilder, RigidBodyBuilder},
};
use winit::window::Window;

pub struct Scene {
    pub renderer: Renderer,
    pub physics: Physics,
    pub audio: AudioManager,
    pub camera_controller: CameraController,
}

impl Scene {
    pub fn new(window: Arc<Window>) -> Self {
        Self {
            renderer: pollster::block_on(Renderer::new(window)).unwrap(),
            physics: Physics::new(),
            audio: AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
            camera_controller: CameraController::default(),
        }
    }

    pub fn add_gltf(&mut self, path: &str) {
        log::info!("Adding gltf to scene");

        let gltf = Gltf::from_slice(&fs::read(path).unwrap()).unwrap();
        let mut instances: HashMap<String, Vec<InstanceRaw>> = HashMap::new();
        let mut textured_meshes: HashMap<String, (Vec<TexturedVertex>, Vec<u16>, Vec<u8>)> =
            HashMap::new();
        let mut colored_meshes: HashMap<String, (Vec<ColoredVertex>, Vec<u16>, [f32; 4])> =
            HashMap::new();
        let mut collider_meshes: HashMap<String, (Vec<Vec3>, Vec<u16>)> = HashMap::new();

        if let Some(blob) = &gltf.blob {
            log::info!("Data collection");
            for node in gltf.nodes() {
                self.add_node(
                    node,
                    blob,
                    &mut instances,
                    &mut textured_meshes,
                    &mut colored_meshes,
                    &mut collider_meshes,
                );
            }
        }

        log::info!("Processing meshes");
        for (name, (vertices, indices, _base_color)) in colored_meshes {
            self.renderer.color_pipeline.add_mesh(
                &self.renderer.device,
                &vertices,
                &indices,
                instances.get(&name).unwrap(),
            );
        }

        for (name, (vertices, indices, image_data)) in textured_meshes {
            let texture = Texture::from_bytes(
                &self.renderer.device,
                &self.renderer.queue,
                &image_data,
                &name,
            )
            .unwrap();

            self.renderer.texture_pipeline.add_mesh(
                &self.renderer.device,
                &texture,
                &vertices,
                &indices,
                instances.get(&name).unwrap(),
            );
        }

        log::info!("Adding physics objects");
        for (name, (positions, indices)) in collider_meshes {
            let instances_list = instances.remove(&name).unwrap();

            for instance in instances_list {
                let model_matrix = Mat4::from_cols_array_2d(&instance.model);
                let (scale, rotation, translation) = model_matrix.to_scale_rotation_translation();
                let angvel = rotation.to_scaled_axis();

                let scaled_vertices: Vec<Vec3> = positions.iter().map(|v| *v * scale).collect();

                let points: Vec<Point<_>> = scaled_vertices
                    .iter()
                    .map(|v| Point::new(v.x, v.y, v.z))
                    .collect();

                let det = model_matrix.determinant();
                let mut final_indices = indices.clone();
                if det < 0.0 {
                    for chunk in final_indices.chunks_exact_mut(3) {
                        chunk.swap(1, 2);
                    }
                }

                let triangles: Vec<[u32; 3]> = final_indices
                    .chunks_exact(3)
                    .map(|tri| [tri[0] as u32, tri[1] as u32, tri[2] as u32])
                    .collect();

                let collider = match ColliderBuilder::trimesh(points, triangles) {
                    Ok(builder) => builder.build(),
                    Err(e) => {
                        log::error!(
                            "Failed to create trimesh collider for mesh {}: {:?}",
                            name,
                            e
                        );
                        continue;
                    }
                };

                let rigid_body = RigidBodyBuilder::fixed()
                    .translation(Vector::new(translation.x, translation.y, translation.z))
                    .rotation(Vector::new(angvel.x, angvel.y, angvel.z))
                    .build();

                let rigid_body_handle = self.physics.bodies.insert(rigid_body);
                self.physics.colliders.insert_with_parent(
                    collider,
                    rigid_body_handle,
                    &mut self.physics.bodies,
                );

                log::info!("RigidBody of {name} created on {translation}");
            }
        }
    }

    pub fn add_node(
        &mut self,
        node: Node,
        blob: &[u8],
        instances: &mut HashMap<String, Vec<InstanceRaw>>,
        textured_meshes: &mut HashMap<String, (Vec<TexturedVertex>, Vec<u16>, Vec<u8>)>,
        colored_meshes: &mut HashMap<String, (Vec<ColoredVertex>, Vec<u16>, [f32; 4])>,
        collider_meshes: &mut HashMap<String, (Vec<Vec3>, Vec<u16>)>,
    ) {
        let Some(mesh) = node.mesh() else { return };
        let Some(name) = mesh.name() else { return };

        let model_matrix = Mat3::from_mat4(Mat4::from_cols_array_2d(&node.transform().matrix()));
        let normal_matrix = model_matrix.inverse().transpose();

        if let Some((base_name, _)) = name.split_once('.') {
            instances
                .entry(base_name.to_string())
                .or_default()
                .push(InstanceRaw {
                    model: node.transform().matrix(),
                    normal: normal_matrix.to_cols_array_2d(),
                });
            return;
        }

        for primitive in mesh.primitives() {
            self.add_primitive(
                primitive,
                name,
                blob,
                textured_meshes,
                colored_meshes,
                collider_meshes,
            );
        }
    }

    pub fn add_primitive(
        &mut self,
        primitive: Primitive,
        name: &str,
        blob: &[u8],
        textured_meshes: &mut HashMap<String, (Vec<TexturedVertex>, Vec<u16>, Vec<u8>)>,
        colored_meshes: &mut HashMap<String, (Vec<ColoredVertex>, Vec<u16>, [f32; 4])>,
        collider_meshes: &mut HashMap<String, (Vec<Vec3>, Vec<u16>)>,
    ) {
        let reader = primitive.reader(|buffer| {
            if buffer.index() == 0 {
                Some(blob)
            } else {
                None
            }
        });

        let indices: Vec<u16> = match reader
            .read_indices()
            .map(|i| i.into_u32().map(|v| v as u16))
        {
            Some(indices) => indices.collect(),
            None => return,
        };

        let positions: Vec<Vec3> = reader.read_positions().unwrap().map(Vec3::from).collect();
        if positions.is_empty() {
            log::warn!("Mesh '{}' has no positions, skipping", name);
            return;
        }

        let normals = match reader.read_normals() {
            Some(n) => n.map(Vec3::from).collect(),
            None => {
                log::warn!("Normals not found for '{}', generating...", name);
                Renderer::compute_normals(&positions, &indices)
            }
        };

        if let Some(tex_coords) = reader
            .read_tex_coords(0)
            .map(|t| t.into_f32().map(Vec2::from).collect::<Vec<_>>())
        {
            log::info!("Finded texture coords of {name}, trying load texture info");

            if let Some(texture_info) = primitive
                .material()
                .pbr_metallic_roughness()
                .base_color_texture()
            {
                log::info!("Texture info loaded, trying get texture");
                if let gltf::image::Source::View { view, .. } =
                    texture_info.texture().source().source()
                {
                    log::info!("Try load texture mesh");

                    let image_data = &blob[view.offset()..view.offset() + view.length()];

                    let vertices = positions
                        .iter()
                        .zip(tex_coords.iter())
                        .zip(normals.iter())
                        .map(|((pos, uv), normal)| TexturedVertex {
                            position: pos.to_array(),
                            tex_coords: [uv.x, uv.y],
                            normal: normal.to_array(),
                        })
                        .collect();

                    collider_meshes.insert(name.to_string(), (positions, indices.clone()));
                    textured_meshes
                        .insert(name.to_string(), (vertices, indices, image_data.to_vec()));
                    return;
                }
            }
        }

        let colors = reader
            .read_colors(0)
            .map(|c| c.into_rgba_f32().map(|v| [v[0], v[1], v[2]]).collect())
            .unwrap_or_else(|| {
                let base = primitive
                    .material()
                    .pbr_metallic_roughness()
                    .base_color_factor();
                vec![[base[0], base[1], base[2]]; positions.len()]
            });

        let vertices = positions
            .iter()
            .zip(colors.iter())
            .zip(normals.iter())
            .map(|((pos, color), normal)| ColoredVertex {
                position: pos.to_array(),
                color: *color,
                normal: normal.to_array(),
            })
            .collect();

        collider_meshes.insert(name.to_string(), (positions, indices.clone()));

        colored_meshes.insert(
            name.to_string(),
            (
                vertices,
                indices,
                primitive
                    .material()
                    .pbr_metallic_roughness()
                    .base_color_factor(),
            ),
        );
    }

    pub fn init_level(&mut self) {
        self.add_gltf("map.glb");

        self.audio
            .play(StaticSoundData::from_file("assets/music/12.ogg").unwrap())
            .unwrap();
    }
}
