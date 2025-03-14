use anyhow::Result;
use game::Game;
use winit::event_loop::EventLoop;

pub mod game;
pub mod renderer;

fn main() -> Result<()> {
    EventLoop::new()?.run_app(&mut Game::default())?;

    Ok(())
}
