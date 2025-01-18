mod state;
mod instance;
mod vertex;
mod uniform;

use state::WgpuState;

use winit::{
    event::*,
    event_loop::{self, EventLoop},
    keyboard::{KeyCode,PhysicalKey},
    window::WindowBuilder
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(target_arch = "wasm32")]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;



fn main() {
    pollster::block_on(run());
}

#[cfg_attr(all(target_arch = "wasm32",target_os = "unknown"), wasm_bindgen(start))]
async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
            wasm_logger::init(wasm_logger::Config::default());
        } else {
            env_logger::init();
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            use winit::platform::web::{
                EventLoopExtWebSys
            };
            let event_loop = EventLoop::new()
                .expect("Event Loop Error");
            let window = WindowBuilder::new()
                .with_title("Pentagrams WebAssembly")
                .build(&event_loop)
                .unwrap();
        } else {
            let event_loop = EventLoop::new()
                .expect("Event Loop Error");
            let window = WindowBuilder::new()
                .with_title("Pentagrams")
                .build(&event_loop)
                .unwrap();
        }
    }     

    cfg_if::cfg_if! {
        if #[cfg(all(target_os = "unknown", target_arch = "wasm32")) ] { 
            use winit::dpi::PhysicalSize;
            use winit::platform::web::WindowExtWebSys;
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("wasm-example")?;
                    let canvas = web_sys::Element::from(window.canvas()?);
                    dst.append_child(&canvas).ok()?;
                    Some(())
                })
                .expect("Couldn't append canvas to document body.");

            let _ = window.request_inner_size(PhysicalSize::new(450, 400));
        } 
    }

    let mut state = WgpuState::new(&window).await;
    let mut surface_configured = false;

    event_loop
    .run(move |event, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window.id() => {
                if !state.input(event) {
                    // UPDATED!
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                                    ..
                                },
                            .. 
                        } => control_flow.exit(),
                        WindowEvent::Resized(physical_size) => {
                            log::info!("physical_size: {physical_size:?}");
                            surface_configured = true;
                            state.resize(*physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            // This tells winit that we want another frame after this one
                            state.window.request_redraw();

                            if !surface_configured {
                                return;
                            }

                            state.update();
                            match state.render() {
                                Ok(_) => {}
                                // Reconfigure the surface if it's lost or outdated
                                Err(
                                    wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                                ) => state.resize(state.size),
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    log::error!("OutOfMemory");
                                    control_flow.exit();
                                }

                                // This happens when the a frame takes too long to present
                                Err(wgpu::SurfaceError::Timeout) => {
                                    log::warn!("Surface timeout")
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    })
    .unwrap();
}