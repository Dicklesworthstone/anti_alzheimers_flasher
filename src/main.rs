use wgpu::{Device, Features, Limits, Queue, Surface, Color};
use winit::{
    event::{Event, WindowEvent, ElementState, VirtualKeyCode, KeyEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder, Fullscreen},
};
use std::sync::atomic::{AtomicUsize, Ordering};

static TIME: AtomicUsize = AtomicUsize::new(0);
const FREQUENCY: f32 = 40.0;  // Frequency of 40Hz
const COLOR1: Color = Color::BLACK; // Can be set to other colors like Color::BLUE
const COLOR2: Color = Color::WHITE; // Can be set to other colors like Color::RED

struct GraphicsContext {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: wgpu::SurfaceConfiguration,
}

impl GraphicsContext {
    async fn new(window: &Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: Features::empty(),
                required_limits: Limits::default(),
            },
            None,
        ).await.unwrap();

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 1,
            alpha_mode: wgpu::AlphaMode::Opaque,
            view_formats: None,
        };
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
        }
    }

    pub fn update_screen(&mut self, use_color1: bool) {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                eprintln!("Failed to acquire next swap chain texture: {:?}", e);
                return;
            }
        };

        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            timestamp_writes: false,
            occlusion_query_set: None,
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(if use_color1 { COLOR1 } else { COLOR2 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
        });

        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}

struct AudioContext {
    stream: Stream,
}

impl AudioContext {
    fn new() -> Self {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("Failed to find an output device");
        let config = device.default_output_config().unwrap();

        let sample_rate = config.sample_rate().0 as f32;
        let channels = config.channels() as usize;

        let stream = device.build_output_stream(
            &StreamConfig::from(config),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                write_sine_wave(data, channels, sample_rate, FREQUENCY)
            },
            |err| eprintln!("an error occurred on stream: {}", err),
            None  // No timeout
        ).unwrap();

        Self { stream }
    }

    pub fn play_audio(&self) {
        self.stream.play().expect("Failed to play audio stream");
    }

    pub fn stop_audio(&self) {
        self.stream.pause().expect("Failed to pause audio stream");
    }
}


fn write_sine_wave(output: &mut [f32], channels: usize, sample_rate: f32, frequency: f32) {
    let max_time = (sample_rate / frequency) as usize;
    let time = TIME.load(Ordering::Relaxed);
    let time_f32 = time as f32 / sample_rate;
    for frame in output.chunks_mut(channels) {
        let value = (time_f32 * frequency * 2.0 * std::f32::consts::PI).sin();
        for sample in frame.iter_mut() {
            *sample = value;
        }
        if time >= max_time {
            TIME.store(0, Ordering::Relaxed);
        } else {
            TIME.fetch_add(1, Ordering::Relaxed);
        }
    }
}


#[tokio::main]
async fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = WindowBuilder::new()
        .with_fullscreen(Some(Fullscreen::Borderless(None)))
        .build(&event_loop)
        .expect("Failed to build window");

    let mut graphics_context = GraphicsContext::new(&window).await;
    let audio_context = AudioContext::new();

    audio_context.play_audio();

    let mut is_white = false;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            Event::WindowEvent {
                event: WindowEvent::KeyboardInput {
                    event: KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        ..
                    },
                    ..
                },
                ..
            } => {
                if let Some(key) = logical_key.to_logical_key() {
                    match key {
                        VirtualKeyCode::Escape => *control_flow = ControlFlow::Exit,
                        _ => {}
                    }
                }
            },

            Event::MainEventsCleared => {
                window.request_redraw();
            },
            Event::RedrawRequested(_) => {
                graphics_context.update_screen(is_white);
                is_white = !is_white;
            },
            _ => {}
        }
    });
}