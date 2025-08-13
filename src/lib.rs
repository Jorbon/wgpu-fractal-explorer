#![allow(dead_code)]

mod common; #[allow(unused_imports)] pub use common::*;

use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use winit::{application::ApplicationHandler, event::{KeyEvent, WindowEvent}, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::{Window, WindowId}};


#[cfg(target_arch = "wasm32")]
pub mod canvas {
    use wasm_bindgen::UnwrapThrowExt;
    use wasm_bindgen::JsCast;
    
    const CANVAS_ID: &str = "canvas";

    pub fn get_canvas() -> web_sys::HtmlCanvasElement {
        let window = web_sys::window().expect_throw("No window!");
        let document = window.document().expect_throw("No document!");
        let canvas = document.get_element_by_id(CANVAS_ID).expect_throw("No canvas!");
        canvas.unchecked_into()
    }
}


pub struct State {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    limits: wgpu::Limits,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();
        
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))] backends: wgpu::Backends::PRIMARY,
            #[cfg(    target_arch = "wasm32" )] backends: wgpu::Backends::GL,
            ..Default::default()
        });
        
        let surface = instance.create_surface(window.clone()).unwrap();
        
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await?;
        
        let limits = adapter.limits();
        
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: limits.clone(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        }).await?;
        
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        Ok(Self {
            window,
            surface,
            limits,
            device,
            queue,
            config,
            is_surface_configured: false,
        })
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width.clamp(1, self.limits.max_texture_dimension_2d);
            self.config.height = height.clamp(1, self.limits.max_texture_dimension_2d);
            
            #[cfg(target_arch = "wasm32")] let canvas = canvas::get_canvas();
            #[cfg(target_arch = "wasm32")] let previous_width = canvas.width();
            #[cfg(target_arch = "wasm32")] let previous_height = canvas.height();
            
            self.surface.configure(&self.device, &self.config);
            
            #[cfg(target_arch = "wasm32")] canvas.set_width(previous_width);
            #[cfg(target_arch = "wasm32")] canvas.set_height(previous_height);
            
            self.is_surface_configured = true;
        }
    }
    
    pub fn update(&mut self) {
        todo!()
    }
    
    pub fn render(&mut self) -> std::result::Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();
        if !self.is_surface_configured { return Ok(()) }
        
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        
        drop(render_pass);
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        Ok(())
    }
}


pub struct App {
    state: Option<State>,
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
}

impl App {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        Self {
            state: None,
            #[cfg(target_arch = "wasm32")] proxy: Some(event_loop.create_proxy()),
        }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();
        
        #[cfg(target_arch = "wasm32")] {
            use winit::platform::web::WindowAttributesExtWebSys;
            window_attributes = window_attributes.with_canvas(Some(canvas::get_canvas()));
        }
        
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        
        #[cfg(not(target_arch = "wasm32"))] {
            self.state = Some(pollster::block_on(State::new(window)).unwrap());
        }
        
        #[cfg(target_arch = "wasm32")]
        if let Some(proxy) = self.proxy.take() {
            wasm_bindgen_futures::spawn_local(async move {
                assert!(proxy.send_event(State::new(window).await.expect("Unable to create canvas.")).is_ok())
            })
        }
    }
    
    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        #[cfg(target_arch = "wasm32")] {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        
        self.state = Some(event);
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let state = match &mut self.state { Some(state) => state, None => return };
        
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size.width, size.height);
            },
            WindowEvent::RedrawRequested => {
                match state.render() {
                    Ok(_) => (),
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => log::error!("Render broke uh oh: {e}")
                }
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent { physical_key: PhysicalKey::Code(code), state, .. }, ..
            } => match (code, state.is_pressed()) {
                (KeyCode::Escape, true) => event_loop.exit(),
                _ => ()
            }
            _ => ()
        }
    }
}


#[cfg(not(target_arch = "wasm32"))]
pub fn run() -> Result<()> {
    env_logger::init();
    log::info!("desktop app started");
    
    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new();
    
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run() -> std::result::Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).unwrap_throw();
    log::info!("wasm app started");
    
    let event_loop = EventLoop::with_user_event().build().unwrap_throw();
    let app = App::new(&event_loop);
    
    // run_app works on wasm, but winit does something goofy with exceptions in it to keep the same return signature.
    // spawn_app does basically the same thing, but without this silliness, so the JS caller returns gracefully.
    use winit::platform::web::EventLoopExtWebSys;
    event_loop.spawn_app(app);
    Ok(())
}



