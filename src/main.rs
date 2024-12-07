use wgpu_3d_viewer::{
    model::Model,
    scene::{Scene, Transform, Renderer},
};
use winit::{
    event::*,
    event_loop::EventLoop,
    window::WindowBuilder,
    keyboard::{KeyCode, PhysicalKey},
};
use std::path::Path;
use glam::Vec3;

const MOVE_SPEED: f32 = 0.1;
const MOUSE_SPEED: f32 = 0.005;
const ZOOM_SPEED: f32 = 0.2;

struct InputState {
    mouse_pressed: bool,
    last_mouse_pos: Option<(f32, f32)>,
}

async fn run() {
    env_logger::init();

    // Create window and event loop
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("3D Viewer")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();

    // Create the scene
    let mut scene = Scene::new(800, 600);

    // Create the surface and renderer
    let instance = wgpu::Instance::default();
    let surface = unsafe {
        instance.create_surface_unsafe(
            wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap()
        )
    }.unwrap();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let surface_caps = surface.get_capabilities(&adapter);
    let size = window.inner_size();
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_caps.formats[0],
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let mut renderer = Renderer::new(&device, &queue, &config);

    // Load the models
    let model1 = Model::load(
        &device,
        &queue,
        Path::new("assets/8b16ddeb-f011-4f13-bab7-615edd40aee9.glb"),
        &renderer.material_bind_group_layout,
    ).expect("Failed to load model 1");

    let model2 = Model::load(
        &device,
        &queue,
        Path::new("assets/cb088356-1d69-41a5-b46d-4bc22aafa1b7.glb"),
        &renderer.material_bind_group_layout,
    ).expect("Failed to load model 2");

    // Add multiple instances of each model with different transforms
    let positions = [
        Vec3::new(-3.0, 0.0, -3.0),
        Vec3::new(3.0, 0.0, -3.0),
        Vec3::new(-3.0, 0.0, 3.0),
        Vec3::new(3.0, 0.0, 3.0),
    ];

    let rotations = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, std::f32::consts::PI * 0.5, 0.0),
        Vec3::new(0.0, std::f32::consts::PI, 0.0),
        Vec3::new(0.0, std::f32::consts::PI * 1.5, 0.0),
    ];

    // Add instances of model1
    for i in 0..2 {
        let mut transform = Transform::new();
        transform.position = positions[i];
        transform.rotation = rotations[i];
        transform.scale = Vec3::splat(1.0);
        scene.add_object(model1.clone_with_device(&device, &queue, &renderer.material_bind_group_layout), transform);
    }

    // Add instances of model2
    for i in 2..4 {
        let mut transform = Transform::new();
        transform.position = positions[i];
        transform.rotation = rotations[i];
        transform.scale = Vec3::splat(1.0);
        scene.add_object(model2.clone_with_device(&device, &queue, &renderer.material_bind_group_layout), transform);
    }

    // Set up lighting
    scene.set_ambient_light(0.2);
    scene.set_directional_light(
        Vec3::new(1.0, 0.9, 0.8), // Warm sunlight color
        Vec3::new(-1.0, -1.0, -0.5).normalize(), // Sun direction
    );

    let mut input = InputState {
        mouse_pressed: false,
        last_mouse_pos: None,
    };

    // Event loop
    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { 
                window_id,
                event: WindowEvent::CloseRequested,
            } if window_id == window.id() => {
                elwt.exit();
            }
            Event::WindowEvent { 
                window_id,
                event: WindowEvent::KeyboardInput {
                    event: KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state: ElementState::Pressed,
                        ..
                    },
                    ..
                },
            } if window_id == window.id() => {
                match key_code {
                    KeyCode::KeyW => scene.camera.move_forward(MOVE_SPEED),
                    KeyCode::KeyS => scene.camera.move_forward(-MOVE_SPEED),
                    KeyCode::KeyA => scene.camera.move_right(-MOVE_SPEED),
                    KeyCode::KeyD => scene.camera.move_right(MOVE_SPEED),
                    KeyCode::Space => scene.camera.move_up(MOVE_SPEED),
                    KeyCode::ShiftLeft => scene.camera.move_up(-MOVE_SPEED),
                    _ => (),
                }
                window.request_redraw();
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::MouseWheel { delta, .. },
            } if window_id == window.id() => {
                let delta_y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };
                scene.camera.zoom(-delta_y * ZOOM_SPEED);
                window.request_redraw();
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                },
            } if window_id == window.id() => {
                input.mouse_pressed = state == ElementState::Pressed;
                if !input.mouse_pressed {
                    input.last_mouse_pos = None;
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CursorMoved { position, .. },
            } if window_id == window.id() => {
                if input.mouse_pressed {
                    let new_pos = (position.x as f32, position.y as f32);
                    if let Some((last_x, last_y)) = input.last_mouse_pos {
                        let delta_x = (new_pos.0 - last_x) * MOUSE_SPEED;
                        let delta_y = (new_pos.1 - last_y) * MOUSE_SPEED;
                        scene.camera.rotate(delta_x, delta_y);
                        window.request_redraw();
                    }
                    input.last_mouse_pos = Some(new_pos);
                }
            }
            Event::WindowEvent { 
                window_id,
                event: WindowEvent::Resized(new_size),
            } if window_id == window.id() => {
                if new_size.width > 0 && new_size.height > 0 {
                    config.width = new_size.width;
                    config.height = new_size.height;
                    surface.configure(&device, &config);
                    scene.resize(new_size.width, new_size.height);
                    renderer.resize(&device, &config);
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::RedrawRequested,
            } if window_id == window.id() => {
                match surface.get_current_texture() {
                    Ok(frame) => {
                        let view = frame.texture.create_view(&Default::default());
                        if let Err(e) = renderer.render(&device, &queue, &view, &scene) {
                            eprintln!("Failed to render: {:?}", e);
                        }
                        frame.present();
                    }
                    Err(e) => {
                        eprintln!("Failed to get current texture: {:?}", e);
                    }
                }
                window.request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}

fn main() {
    pollster::block_on(run());
} 