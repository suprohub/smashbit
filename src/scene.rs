use std::{
    collections::HashMap,
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

use crate::{
    camera_controller::CameraController,
    physics::Physics,
    renderer::{
        Renderer,
        pipeline::{
            InstanceRaw,
            color::{ColoredVertex, generate_sphere},
            texture::TexturedVertex,
        },
        texture::Texture,
    },
};
use bimap::BiHashMap;
use glam::{Mat3, Mat4, Vec2, Vec3};
use gltf::{Gltf, Node, Primitive};
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, sound::static_sound::StaticSoundData,
};
use rapier3d::{
    math::{Point, Vector},
    prelude::{ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle},
};
use winit::window::Window;

pub struct Scene {
    pub renderer: Renderer,
    pub physics: Physics,
    pub audio: AudioManager,
    pub camera_controller: CameraController,
    pub objects: BiHashMap<u128, (RigidBodyHandle, ColliderHandle)>,
}

impl Scene {
    pub fn new(window: Arc<Window>) -> Self {
        Self {
            renderer: pollster::block_on(Renderer::new(window)).unwrap(),
            physics: Physics::new(),
            audio: AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
            camera_controller: CameraController::default(),
            objects: BiHashMap::new(),
        }
    }

    pub fn cull_instances_behind_camera(&mut self) {
        let camera_position = self.renderer.camera.position;
        let camera_forward = self.renderer.camera.calc_view_dir();

        let mut to_remove = Vec::new();

        for (mesh_id, mesh) in self.renderer.color_pipeline.meshes.iter().chain(
            self.renderer
                .texture_pipeline
                .meshes
                .iter()
                .map(|(id, (mesh, _))| (id, mesh)),
        ) {
            for (instance_index, instance) in mesh.instances.iter().enumerate() {
                let model = instance.model;
                let position = glam::Vec3::new(model[3][0], model[3][1], model[3][2]);

                let to_object = position - camera_position;
                if to_object.dot(camera_forward) < 0.0 {
                    to_remove.push((*mesh_id, instance_index));
                }
            }
        }

        for (mesh_id, instance_index) in to_remove.into_iter().rev() {
            self.remove_instance(mesh_id, instance_index);
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
                hash_string_to_u64(&name),
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
                hash_string_to_u64(&name),
                &texture,
                &vertices,
                &indices,
                instances.get(&name).unwrap(),
            );
        }

        log::info!("Adding physics objects");
        for (name, (positions, indices)) in collider_meshes {
            let instances_list = instances.remove(&name).unwrap();

            for (instance_index, instance) in instances_list.into_iter().enumerate() {
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
                        log::error!("Failed to create trimesh collider for mesh {name}: {e:?}");
                        continue;
                    }
                };

                let rigid_body = RigidBodyBuilder::fixed()
                    .translation(Vector::new(translation.x, translation.y, translation.z))
                    .rotation(Vector::new(angvel.x, angvel.y, angvel.z))
                    .build();

                let rigid_body_handle = self.physics.bodies.insert(rigid_body);
                let collider_handle = self.physics.colliders.insert_with_parent(
                    collider,
                    rigid_body_handle,
                    &mut self.physics.bodies,
                );

                let mesh_id = hash_string_to_u64(&name);
                let id = ((mesh_id as u128) << 64) | (instance_index as u128);

                self.objects
                    .insert(id, (rigid_body_handle, collider_handle));

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
            log::warn!("Mesh '{name}' has no positions, skipping");
            return;
        }

        let normals = match reader.read_normals() {
            Some(n) => n.map(Vec3::from).collect(),
            None => {
                log::warn!("Normals not found for '{name}', generating...");
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

    pub fn init_ball(&mut self) {
        let (vertices, indices) = generate_sphere(0.5, 16, 16, [1.0, 0.0, 0.0]);
        self.renderer.color_pipeline.add_mesh(
            &self.renderer.device,
            hash_string_to_u64("ball"),
            &vertices,
            &indices,
            &[InstanceRaw {
                model: Default::default(),
                normal: Default::default(),
            }],
        );
    }

    pub fn remove_instance(&mut self, mesh_id: u64, instance_index: usize) {
        if let Some(mesh) = self
            .renderer
            .color_pipeline
            .meshes
            .get_mut(&mesh_id)
            .or(self
                .renderer
                .texture_pipeline
                .meshes
                .get_mut(&mesh_id)
                .map(|(m, _)| m))
        {
            let instance_count_before = mesh.instances.len();

            if instance_index >= instance_count_before {
                return;
            }

            let last_index = instance_count_before - 1;

            mesh.remove_instance(&self.renderer.device, &self.renderer.queue, instance_index);

            let user_data_removed = ((mesh_id as u128) << 64) | (instance_index as u128);

            if let Some((_, (rigid_body, collider))) =
                self.objects.remove_by_left(&user_data_removed)
            {
                self.physics.colliders.remove(
                    collider,
                    &mut self.physics.islands,
                    &mut self.physics.bodies,
                    true,
                );
                self.physics.bodies.remove(
                    rigid_body,
                    &mut self.physics.islands,
                    &mut self.physics.colliders,
                    &mut self.physics.impulse_joints,
                    &mut self.physics.multibody_joints,
                    true,
                );
            }

            if instance_index != last_index {
                let old_user_data_last = ((mesh_id as u128) << 64) | (last_index as u128);
                if let Some((_, (rigid_body, a))) = self.objects.remove_by_left(&old_user_data_last)
                {
                    if let Some(body) = self.physics.bodies.get_mut(rigid_body) {
                        body.user_data = user_data_removed;
                    }
                    self.objects.insert(user_data_removed, (rigid_body, a));
                }
            }
        }
    }

    pub fn spawn_ball_instance(
        &mut self,
        position: Vec3,
        direction: Vec3,
        speed: f32,
        radius: f32,
    ) {
        let mesh_id = hash_string_to_u64("ball");
        let mesh = self
            .renderer
            .color_pipeline
            .meshes
            .get_mut(&mesh_id)
            .unwrap();

        let transform = Mat4::from_translation(position);
        let normal_matrix = Mat3::from_mat4(transform).inverse().transpose();

        mesh.add_instance(
            &self.renderer.device,
            &self.renderer.queue,
            &InstanceRaw {
                model: transform.to_cols_array_2d(),
                normal: normal_matrix.to_cols_array_2d(),
            },
        );

        let instance_id = mesh.instances.len() as u128 - 1;
        let id = ((mesh_id as u128) << 64) | instance_id;

        let velocity = direction * speed;
        self.objects
            .insert(id, self.physics.create_ball(id, position, velocity, radius));
    }

    pub fn update_objects(&mut self) {
        for (_, body) in self.physics.bodies.iter() {
            // Update dynamic objects
            if body.user_data != 0 {
                let model = body.position().to_homogeneous().into();
                let normal = Mat3::from_mat4(Mat4::from_cols_array_2d(&model))
                    .inverse()
                    .transpose()
                    .to_cols_array_2d();

                self.renderer
                    .color_pipeline
                    .meshes
                    .get_mut(&((body.user_data >> 64) as u64))
                    .unwrap()
                    .update_instance(
                        &self.renderer.queue,
                        body.user_data as usize,
                        &InstanceRaw { model, normal },
                    );
            }
        }
    }

    pub fn init_level(&mut self) {
        self.init_ball();
        self.add_gltf("map.glb");

        self.audio
            .play(StaticSoundData::from_file("assets/music/12.ogg").unwrap())
            .unwrap();
    }
}

fn hash_string_to_u64(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}
