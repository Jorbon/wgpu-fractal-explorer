#![allow(dead_code)]

mod common; #[allow(unused_imports)] pub use common::*;
mod math; #[allow(unused_imports)] pub use math::*;
mod teapot; #[allow(unused_imports)] pub use teapot::*;

use std::sync::Arc;

use winit::{application::ApplicationHandler, dpi::{PhysicalPosition, PhysicalSize}, event::{KeyEvent, WindowEvent}, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::{Window, WindowId}};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;



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


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: &[wgpu::VertexAttribute] = &[
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x3,
        },
    ];
    
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

type Index = u32;


const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.1,  0.5,  0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [-0.5,  0.0,  0.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [-0.3, -0.5,  0.0], color: [0.0, 0.0, 1.0] },
    Vertex { position: [ 0.4, -0.4,  0.0], color: [1.0, 0.5, 0.0] },
    Vertex { position: [ 0.5,  0.2,  0.0], color: [0.5, 0.0, 1.0] },
];

const INDICES: &[Index] = &[0, 1, 2, 0, 2, 3, 0, 3, 4];





struct Camera {
    position: Vec3<f32>,
    yaw: f32,
    pitch: f32,
    roll: f32,
    
}



pub struct State {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    limits: wgpu::Limits,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    render_pipeline: wgpu::RenderPipeline,
    
    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    viewport: glyphon::Viewport,
    atlas: glyphon::TextAtlas,
    text_renderer: glyphon::TextRenderer,
    text_buffer: glyphon::Buffer,
    
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    camera: Camera,
    mouse_position: PhysicalPosition<f64>,
    
    average_frame_dt: f32,
    previous_frame_time: std::time::Instant,
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
        
        #[cfg(not(target_arch = "wasm32"))]
        let required_limits = wgpu::Limits::default();
        #[cfg(target_arch = "wasm32")]
        let required_limits = wgpu::Limits::downlevel_webgl2_defaults();
        
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits,
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
        
        
        let image_bytes = include_bytes!("../assets/test.png");
        let image = image::load_from_memory(image_bytes).unwrap().to_rgba8();
        let (image_width, image_height) = image.dimensions();
        
        let texture_size = wgpu::Extent3d {
            width: image_width,
            height: image_height,
            depth_or_array_layers: 1,
        };
        
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("texture"),
            view_formats: &[],
        });
        
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * image_width),
                rows_per_image: Some(image_height),
            },
            texture_size,
        );
        
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        
        
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniforms"),
            contents: bytemuck::cast_slice(&[0.0f32, 0.0, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("uniform bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform bind group"),
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });
        
        
        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(teapot::VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice(teapot::INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into())
        });
        
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render pipeline layout"),
            bind_group_layouts: &[
                &bind_group_layout,
                &uniform_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    Vertex::desc(),
                ],
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[("test_constant", 0.9)],
                    zero_initialize_workgroup_memory: false,
                },
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[("test_constant", 0.9)],
                    zero_initialize_workgroup_memory: false,
                },
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        
        
        let mut font_system = glyphon::FontSystem::new();
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(&device);
        let viewport = glyphon::Viewport::new(&device, &cache);
        let mut atlas = glyphon::TextAtlas::new(&device, &queue, &cache, surface_format);
        let text_renderer = glyphon::TextRenderer::new(&mut atlas, &device, wgpu::MultisampleState::default(), None);
        let mut text_buffer = glyphon::Buffer::new(&mut font_system, glyphon::Metrics { font_size: 16.0, line_height: 16.0 });
        text_buffer.set_size(&mut font_system, Some(300.0), Some(100.0));
        text_buffer.set_text(&mut font_system, "Text text!", &glyphon::Attrs::new().color(glyphon::Color::rgb(255, 255, 255)), glyphon::Shaping::Basic);
        text_buffer.shape_until_scroll(&mut font_system, false);
        
        
        Ok(Self {
            window,
            surface,
            limits,
            device,
            queue,
            config,
            is_surface_configured: false,
            render_pipeline,
            
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            text_buffer,
            
            vertex_buffer,
            index_buffer,
            bind_group,
            uniform_buffer,
            uniform_bind_group,
            camera: Camera { position: Vec3(0.0, 0.0, 0.0), yaw: 0.0, pitch: 0.0, roll: 0.0 },
            mouse_position: PhysicalPosition { x: 0.0, y: 0.0 },
            
            average_frame_dt: 0.0,
            previous_frame_time: std::time::Instant::now(),
        })
    }
    
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 { return }
        self.config.width = new_size.width.min(self.limits.max_texture_dimension_2d);
        self.config.height = new_size.height.min(self.limits.max_texture_dimension_2d);
        // Make sure canvas width and height are set in CSS or this call will take control and crash the app in a very silly way!
        self.surface.configure(&self.device, &self.config);
        self.is_surface_configured = true;
        
        self.viewport.update(&self.queue, glyphon::Resolution { width: self.config.width, height: self.config.height });
    }
    
    pub fn update(&mut self) {
        todo!()
    }
    
    pub fn render(&mut self) -> std::result::Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();
        if !self.is_surface_configured { return Ok(()) }
        
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.previous_frame_time).as_secs_f32();
        self.previous_frame_time = now;
        self.average_frame_dt = 0.99 * self.average_frame_dt + 0.01 * dt;
        
        self.text_buffer.set_text(&mut self.font_system, &format!("Fps: {}", 1.0 / self.average_frame_dt), &glyphon::Attrs::new().color(glyphon::Color::rgb(255, 255, 255)), glyphon::Shaping::Basic);
        
        
        self.text_renderer.prepare(&self.device, &self.queue, &mut self.font_system, &mut self.atlas, &self.viewport, [glyphon::TextArea {
            buffer: &self.text_buffer,
            left: 10.0,
            top: 10.0,
            scale: self.window.scale_factor() as f32,
            bounds: glyphon::TextBounds { left: 10, top: 10, right: self.config.width as i32 - 10, bottom: self.config.height as i32 - 10, },
            default_color: glyphon::Color::rgb(255, 255, 255),
            custom_glyphs: &[],
        }], &mut self.swash_cache).unwrap();
        
        
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..(teapot::INDICES.len() as u32 * 3), 0, 0..1);
        
        self.text_renderer.render(&self.atlas, &self.viewport, &mut render_pass).unwrap();
        
        drop(render_pass);
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        self.atlas.trim();
        
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
            event.resize(event.window.inner_size());
        }
        
        self.state = Some(event);
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let state = match &mut self.state { Some(state) => state, None => return };
        
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size),
            
            WindowEvent::RedrawRequested => match state.render() {
                Ok(_) => (),
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    state.resize(state.window.inner_size());
                }
                Err(e) => log::error!("Render broke uh oh: {e}")
            }
            
            WindowEvent::KeyboardInput {
                event: KeyEvent { physical_key: PhysicalKey::Code(code), state, .. }, ..
            } => match (code, state.is_pressed()) {
                (KeyCode::Escape, true) => event_loop.exit(),
                _ => ()
            }
            
            WindowEvent::CursorMoved { position, device_id: _ } => {
                state.mouse_position = position;
                state.queue.write_buffer(&state.uniform_buffer, 0, bytemuck::cast_slice(&[state.mouse_position.x as f32 / state.config.width as f32, state.mouse_position.y as f32 / state.config.height as f32]));
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



