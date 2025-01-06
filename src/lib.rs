mod state;
use std::{f32::consts::E, sync::Arc};
use log::Level;
use state::WgpuState;
use wasm_bindgen::JsCast;
use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::{ControlFlow, EventLoop}, window::Window};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(target_arch = "wasm32")]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


#[derive(Default)]
pub struct App<'a> {
    window: Option<Arc<Window>>,
    wgpu_state: Option<WgpuState<'a>>,
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            #[cfg(target_arch = "wasm32")]
            {
                use wgpu::web_sys;
                use winit::platform::web::WindowAttributesExtWebSys;

                let window_attr = Window::default_attributes()
                    .with_canvas(
                        Some(gloo::utils::document()
                            .get_element_by_id("canvas")
                            .unwrap().dyn_into::<web_sys::HtmlCanvasElement>().unwrap()
                        ));
                let window = Arc::new(
                    event_loop
                        .create_window(window_attr)
                        .expect("Failed to create window"),
                );

                self.window = Some(window.clone());    
                self.wgpu_state = Some(WgpuState::new_wasm(self.window.clone().unwrap()));
                self.wgpu_state
                    .as_mut()
                    .unwrap()
                    .wasm_runtime_setup();

            }
            #[cfg(not(target_arch = "wasm32"))] {
                
                let window_attr = Window::default_attributes()
                    .with_title("Pentagrams");
                let window = Arc::new(
                    event_loop
                        .create_window(window_attr)
                        .expect("Failed to create window"),
                );
                self.window = Some(window.clone());
                
                let wgpu_state = WgpuState::native_new(self.window.clone().unwrap());
                self.wgpu_state = Some(wgpu_state);
            }
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
                    wgpu_state.render();
                    self.window.as_ref().unwrap().request_redraw();
                }
                _ => {}
            }
        }
    }
}


fn main() {
    pollster::block_on(run());
}

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            wasm_logger::init(wasm_logger::Config::default());

            use wasm_bindgen::JsCast;
            use winit::platform::web::EventLoopExtWebSys;
            let event_loop = EventLoop::builder().build().unwrap();
        } else {
            env_logger::init();
            let event_loop = EventLoop::builder().build().unwrap();
        }
    }


    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop
        .set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    
    let _ = event_loop.run_app(&mut app);
}