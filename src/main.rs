#![feature(async_await)]
#[macro_use]
extern crate gullery_macros;

use gullery::ContextState;
use gullery::buffer::*;
use gullery::framebuffer::{*, render_state::*};
use gullery::program::*;
use gullery::image_format::Rgba;
use gullery::vertex::VertexArrayObject;

use cgmath_geometry::cgmath;
use cgmath_geometry::rect::{OffsetBox};

use cgmath::*;
use glutin::{
    ContextBuilder, GlRequest,
    event::{DeviceEvent, ElementState, WindowEvent, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    dpi::LogicalSize,
    window::WindowBuilder,
};
use winit_async::{EventLoopAsync, EventAsync as Event};
use std::time::Instant;

#[derive(Vertex, Clone, Copy)]
struct Vertex {
    pos: Point3<f32>,
}

#[derive(Clone, Copy, Uniforms)]
struct Uniforms {
    matrix: Matrix4<f32>,
}

// USAGE INSTRUCTIONS
// - Click the window to take mouse control over the cube. This will rotate the cube with
//   the mouse, and will only redraw when the mouse is moved.
// - Press escape to release mouse control. This will automatically rotate the cube at one
//   degree per frame.

fn main() {
    let event_loop = EventLoop::new();
    let wb = WindowBuilder::new().with_visible(false).with_inner_size(LogicalSize::new(512.0, 512.0));

    // Set this to false to uncap the framerate.
    let vsync = true;

    let window = unsafe {
        ContextBuilder::new()
            .with_gl(GlRequest::GlThenGles {
                opengl_version: (3, 3),
                opengles_version: (3, 0)
            })
            .with_srgb(true)
            .with_vsync(vsync)
            .build_windowed(wb, &event_loop)
            .unwrap()
            .make_current()
            .unwrap()
    };
    let state = unsafe{ ContextState::new(|addr| window.context().get_proc_address(addr)) };

    let w = 0.5;
    let vertex_buffer = Buffer::with_data(BufferUsage::StaticDraw, &[
        Vertex {
            pos: Point3::new(w, -w, -w)
        },
        Vertex {
            pos: Point3::new(w, -w, w)
        },
        Vertex {
            pos: Point3::new(-w, -w, w)
        },
        Vertex {
            pos: Point3::new(-w, -w, -w)
        },
        Vertex {
            pos: Point3::new(w, w, -w)
        },
        Vertex {
            pos: Point3::new(w, w, w)
        },
        Vertex {
            pos: Point3::new(-w, w, w)
        },
        Vertex {
            pos: Point3::new(-w, w, -w)
        },
    ], state.clone());
    let index_buffer = Buffer::with_data(BufferUsage::StaticDraw, &[
        0, 3, 2,
        0, 2, 1,
        4, 5, 6,
        4, 6, 7,
        0, 1, 5,
        0, 5, 4,
        1, 2, 6,
        1, 6, 5,
        2, 3, 7,
        2, 7, 6,
        4, 7, 3,
        4, 3, 0u16,
    ], state.clone());
    let vao = VertexArrayObject::new(vertex_buffer, Some(index_buffer));
    println!("vao created");

    let vertex_shader = Shader::new(VERTEX_SHADER, state.clone()).unwrap();
    let fragment_shader = Shader::new(FRAGMENT_SHADER, state.clone()).unwrap();
    let (program, warning) = Program::new(&vertex_shader, None, &fragment_shader).unwrap();
    for w in warning {
        println!("{:?}", w);
    }

    let mut render_state = RenderState {
        srgb: true,
        texture_cubemap_seamless: true,
        cull: Some((CullFace::Front, FrontFace::CounterCw)),
        viewport: OffsetBox {
            origin: Point2::new(0, 0),
            dims: Vector2::new(512, 512)
        },
        ..RenderState::default()
    };

    let z_near = 0.1;
    let z_far = 10.0;
    let fov: f32 = 70.0;

    let mut default_framebuffer = FramebufferDefault::new(state.clone()).unwrap();
    let mut rotation = Euler::new(Deg(0.0), Deg(0.0), Deg(0.0));

    let mut aspect_ratio = 1.0;
    let mut window_focused = false;
    let mouse_sensitivity = 0.1;
    let mut use_perspective = true;
    window.window().set_visible(true);
    window.window().request_redraw();

    let mut last_frame = Instant::now();
    event_loop.run_async(async move |mut runner| {
        loop {
            if window_focused {
                println!("wait");
                runner.wait().await;
            }

            println!("recv_events");
            let mut recv_events = runner.recv_events().await;
            println!("next");
            while let Some(event) = recv_events.next().await {
                match event {
                    Event::WindowEvent{event, ..} => match event {
                        WindowEvent::Resized(d) => {
                            aspect_ratio = (d.width / d.height) as f32;
                        },
                        WindowEvent::Focused(f) => {
                            window_focused = f;
                            if f {
                                window.window().set_cursor_grab(true).ok();
                                window.window().set_cursor_visible(false);
                            } else {
                                window.window().set_cursor_grab(false).ok();
                                window.window().set_cursor_visible(true);
                            }
                        },
                        WindowEvent::MouseInput{state: ElementState::Pressed, ..} => {
                            window.window().set_cursor_grab(true).ok();
                            window.window().set_cursor_visible(false);
                            window_focused = true;
                            window.window().request_redraw();
                        }
                        WindowEvent::KeyboardInput{input, ..}
                            if input.state == ElementState::Pressed
                        => {
                            match input.virtual_keycode {
                                Some(VirtualKeyCode::Escape) => {
                                    window.window().set_cursor_grab(false).ok();
                                    window.window().set_cursor_visible(true);
                                    window_focused = false;
                                },
                                Some(VirtualKeyCode::Space) => {
                                    use_perspective = !use_perspective;
                                }
                                _ => (),
                            }
                            window.window().request_redraw();
                        },
                        WindowEvent::CloseRequested => return,
                        _ => ()
                    },
                    Event::DeviceEvent{event, ..} => match event {
                        DeviceEvent::MouseMotion{delta} if window_focused => {
                            rotation.x.0 += delta.1 as f32 * mouse_sensitivity;
                            rotation.y.0 += delta.0 as f32 * mouse_sensitivity;
                            window.window().request_redraw();
                        },
                        _ => ()
                    },
                    _ => (),
                }
            }

            if !window_focused {
                rotation.y.0 += 1.0;
                window.window().request_redraw();
                println!("request redraw");
            }

            println!("redraw_requests");
            let mut redraw_requests = recv_events.redraw_requests().await;
            println!("next");
            while let Some(window_id) = redraw_requests.next().await {
                println!("redraw {:?}", window_id);
                if window_id == window.window().id() {
                    let physical_size = window.window().inner_size().to_physical(window.window().hidpi_factor());
                    render_state.viewport = OffsetBox::new2(0, 0, physical_size.width as u32, physical_size.height as u32);
                    let scale = 1.0 / (fov.to_radians() / 2.0).tan();
                    let perspective_matrix = match use_perspective {
                        true => Matrix4::new(
                            scale / aspect_ratio, 0.0  , 0.0                                       , 0.0,
                            0.0                 , scale, 0.0                                       , 0.0,
                            0.0                 , 0.0  , (z_near + z_far) / (z_near - z_far)       , -1.0,
                            0.0                 , 0.0  , (2.0 * z_far * z_near) / (z_near - z_far) , 0.0
                        ),
                        false => Matrix4::new(
                            scale / aspect_ratio, 0.0  , 0.0                                 , 0.0,
                            0.0                 , scale, 0.0                                 , 0.0,
                            0.0                 , 0.0  , (z_near + z_far) / (z_near - z_far) , -1.0,
                            0.0                 , 0.0  , 0.0                                 , 1.0
                        )
                    } ;
                    let translation_matrix = Matrix4::new(
                        1.0, 0.0, 0.0, 0.0,
                        0.0, 1.0, 0.0, 0.0,
                        0.0, 0.0, 1.0, 0.0,
                        0.0, 0.0, -3.0, 1.0,
                    );
                    let uniform = Uniforms {
                        matrix: perspective_matrix * translation_matrix * Matrix4::from(Matrix3::from(Basis3::from(Quaternion::from(rotation)))),
                    };

                    default_framebuffer.clear_depth(1.0);

                    let clear_color = match window_focused {
                        true => Rgba::new(1.0, 1.0, 1.0, 1.0),
                        false => Rgba::new(0.75, 0.75, 0.75, 1.0),
                    };
                    default_framebuffer.clear_color_all(clear_color);
                    default_framebuffer.draw(DrawMode::Triangles, .., &vao, &program, uniform, render_state);

                    window.swap_buffers().unwrap();

                    // let now = Instant::now();
                    // let framerate = 1.0 / ((now - last_frame).as_millis() as f64 / 1_000.0);
                    // println!("framerate: {}", framerate);
                    // last_frame = now;
                }
            }
        }
    });
}

const VERTEX_SHADER: &str = r#"
    #version 330

    in vec3 pos;
    uniform mat4 matrix;
    out vec3 tc;

    void main() {
        tc = pos;
        gl_Position = matrix * vec4(pos, 1.0);
    }
"#;

const FRAGMENT_SHADER: &str = r#"
    #version 330

    in vec3 tc;
    out vec4 color;

    void main() {
        color = vec4(tc, 1.0);
    }
"#;

