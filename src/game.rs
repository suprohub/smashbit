use std::sync::Arc;

use winit::{application::ApplicationHandler, event::WindowEvent, window::WindowAttributes};

use crate::renderer::Renderer;

#[derive(Default)]
pub struct Game {
    renderer: Option<Renderer>,
}

impl ApplicationHandler for Game {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );
        self.renderer = Some(pollster::block_on(Renderer::new(window)).unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &self.renderer {
                    renderer.window.request_redraw();
                    
                    renderer.render().unwrap();
                }
            }
            _ => {}
        }
    }
}
