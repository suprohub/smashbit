use glam::Vec3;
use rapier3d::{
    math::Vector,
    na::Vector3,
    prelude::{
        BroadPhaseMultiSap, CCDSolver, ColliderBuilder, ColliderHandle, ColliderSet,
        ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, NarrowPhase,
        PhysicsPipeline, QueryPipeline, RigidBodyBuilder, RigidBodyHandle, RigidBodySet,
    },
};

pub struct Physics {
    pub pipeline: PhysicsPipeline,
    pub gravity: Vec3,
    pub integration_parameters: IntegrationParameters,
    pub islands: IslandManager,
    pub broad_phase: BroadPhaseMultiSap,
    pub narrow_phase: NarrowPhase,
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: Option<QueryPipeline>,
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
}

impl Physics {
    pub fn new() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: Vec3::default(),
            integration_parameters: IntegrationParameters::tgs_soft(),
            islands: IslandManager::new(),
            broad_phase: BroadPhaseMultiSap::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: None,
        }
    }

    pub fn step(&mut self, delta_secs: f32, max_dt: f32, time_scale: f32, substeps: u8) {
        self.integration_parameters.dt = (delta_secs * time_scale).min(max_dt);

        let mut substep_integration_parameters = self.integration_parameters;
        substep_integration_parameters.dt /= substeps as f32;

        for _ in 0..substeps {
            self.pipeline.step(
                &Vector3::new(self.gravity.x, self.gravity.y, self.gravity.z),
                &substep_integration_parameters,
                &mut self.islands,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.bodies,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                &mut self.ccd_solver,
                self.query_pipeline.as_mut(),
                &(),
                &(),
            );
        }
    }

    pub fn create_ball(
        &mut self,
        id: u128,
        position: Vec3,
        velocity: Vec3,
        radius: f32,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let rigid_body = self.bodies.insert(
            RigidBodyBuilder::dynamic()
                .translation(Vector::new(position.x, position.y, position.z))
                .linvel(Vector::new(velocity.x, velocity.y, velocity.z))
                .user_data(id)
                .build(),
        );

        let collider = self.colliders.insert_with_parent(
            ColliderBuilder::ball(radius).density(1.0).build(),
            rigid_body,
            &mut self.bodies,
        );

        (rigid_body, collider)
    }
}
