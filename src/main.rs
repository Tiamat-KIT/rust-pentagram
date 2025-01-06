mod state;
use std::sync::Arc;
use state::WgpuState;
use winit::{application::ApplicationHandler, error::EventLoopError, event::{ElementState, WindowEvent}, event_loop::{ControlFlow, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::Window};
use std::result::Result;


#[derive(Default)]
pub struct App<'a> {
    window: Option<Arc<Window>>,
    wgpu_state: Option<WgpuState<'a>>,
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            let window_attr = Window::default_attributes()
                .with_title("Pentagrams");
            let window = Arc::new(
                event_loop
                    .create_window(window_attr)
                    .expect("Failed to create window"),
            );
            self.window = Some(window.clone());

            let wgpu_state = WgpuState::new(window.clone());
            self.wgpu_state = Some(wgpu_state);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(wgpu_state) = self.wgpu_state.as_mut() {
            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                WindowEvent::Resized(size) => {
                    if let Some(window) = self.window.as_ref() {
                        wgpu_state.resize(size);
                        window.request_redraw();
                    }
                }
                WindowEvent::RedrawRequested => {
                    match wgpu_state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            wgpu_state.resize(wgpu_state.size);
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            eprintln!("Out of memory");
                            event_loop.exit();
                        }
                        Err(e) => {
                            eprintln!("Failed to render: {:?}", e);
                        }
                        _ => {}
                    };
                    self.window.as_ref().unwrap().request_redraw();
                }
                _ => {}
            }
        }
    }
}


fn main() -> Result<(), EventLoopError> {
    let event_loop = EventLoop::builder().build()?;

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop
        .set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    let run_app =  event_loop.run_app(&mut app);
    run_app
}