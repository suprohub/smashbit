use anyhow::Result;
use game::Game;
use log::Level;
use winit::event_loop::EventLoop;

pub mod camera_controller;
pub mod game;
pub mod physics;
pub mod renderer;

fn main() -> Result<()> {
    simple_logger::init_with_level(Level::Info)?;
    EventLoop::new()?.run_app(&mut Game::default())?;

    Ok(())
}
