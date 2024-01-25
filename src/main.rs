use cpal::{Stream, StreamConfig, traits::{DeviceTrait, HostTrait, StreamTrait}};
use wgpu::{Device, Features, Limits, Queue, Surface, TextureUsages, Color, util::DeviceExt};
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


struct GraphicsContext<'a> {
    surface: Surface<'a>,
    device: Device,
    queue: Queue,
}

impl<'a> GraphicsContext<'a> {
    async fn new(window: &'a Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(window) }.expect("Failed to create surface");

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: Features::empty(),
                required_limits: Limits::downlevel_webgl2_defaults(),
            },
            None,
        ).await.unwrap();

        // Use get_default_config for setting up the surface
        let config = surface.get_default_config(&adapter, size.width, size.height)
            .expect("Failed to get default surface configuration");

        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
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
    
        {
            let color_attachment = wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,  // Set to None or another texture view if using multisampling
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(if use_color1 { COLOR1 } else { COLOR2 }),
                    store: wgpu::StoreOp::Store, // Changed to StoreOp::Store
                },
            };
    
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(color_attachment)], // Note the use of Some() and array
                depth_stencil_attachment: None,
                timestamp_writes: None, // Add if using timestamp query features
                occlusion_query_set: None, // Add if using occlusion query features
            });
        }
    
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
                        ..
                    },
                    ..
                },
                ..
            } => {
                // Exit the program when any key is pressed
                *control_flow = ControlFlow::Exit;
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
