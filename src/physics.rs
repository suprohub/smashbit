use glam::Vec3;
use rapier3d::{
    na::Vector3,
    prelude::{
        BroadPhaseMultiSap, CCDSolver, ColliderSet, ImpulseJointSet, IntegrationParameters,
        IslandManager, MultibodyJointSet, NarrowPhase, PhysicsPipeline, QueryPipeline,
        RigidBodySet,
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

    pub fn step(&mut self) {
        self.pipeline.step(
            &Vector3::new(self.gravity.x, self.gravity.y, self.gravity.z),
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            self.query_pipeline.as_mut(),
            &mut (),
            &mut (),
        );
    }
}
