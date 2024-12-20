use winit::{
    event::*,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};
use wgpu_3d_viewer::State;

fn main() {
    let event_loop = winit::event_loop::EventLoop::new()
        .expect("Failed to create event loop");
    
    let window = WindowBuilder::new()
        .with_title("3D Engine")
        .with_visible(true)
        .build(&event_loop)
        .unwrap();

    let mut state = State::new(window);
    let mut mouse_captured = false;

    event_loop.run(move |event, window_target| {
        match event {
            Event::WindowEvent { window_id, event } if window_id == state.window().id() => {
                match event {
                    WindowEvent::KeyboardInput {
                        event: KeyEvent {
                            physical_key: PhysicalKey::Code(key_code),
                            state: key_state,
                            ..
                        },
                        ..
                    } => {
                        let pressed = key_state == ElementState::Pressed;
                        match key_code {
                            KeyCode::Escape => {
                                if pressed {
                                    mouse_captured = false;
                                    state.window().set_cursor_grab(winit::window::CursorGrabMode::None)
                                        .unwrap();
                                    state.window().set_cursor_visible(true);
                                }
                            }
                            _ => state.scene.process_keyboard(key_code, pressed),
                        }
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } => {
                        mouse_captured = true;
                        state.window().set_cursor_grab(winit::window::CursorGrabMode::Confined)
                            .or_else(|_e| state.window().set_cursor_grab(winit::window::CursorGrabMode::Locked))
                            .unwrap();
                        state.window().set_cursor_visible(false);
                    }
                    WindowEvent::CloseRequested => {
                        window_target.exit();
                    }
                    WindowEvent::Resized(new_size) => {
                        if new_size.width > 0 && new_size.height > 0 {
                            state.resize(new_size.width, new_size.height);
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        state.render().unwrap();
                    }
                    _ => {}
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } if mouse_captured => {
                state.scene.process_mouse(delta.0 as f32, delta.1 as f32);
            }
            Event::AboutToWait => {
                state.scene.update();
                state.window().request_redraw();
            }
            _ => {}
        }
    }).unwrap();
} 