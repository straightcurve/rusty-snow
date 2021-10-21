mod networking;
mod recording;

use std::env;
use std::thread;
use std::time::Duration;
use std::u64;

#[macro_use]
extern crate glium;

#[derive(Copy, Clone)]
struct SnowVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    if args.len() != 1 && args[1] == "host" {
        let host = networking::Host::new();

        let display = recording::open_display();
        loop {
            let xid = u64::from_str_radix(&args[2], 16).unwrap();
            let mut image = recording::record_linux(display, xid);

            host.send_frame(image.width, image.height, &image.data);

            image.free();
            thread::sleep(Duration::from_millis(16));
        }
    } else {
        let context = zmq::Context::new();
        let subscriber = context.socket(zmq::SUB).unwrap();
        subscriber
            .connect("tcp://localhost:5563")
            .expect("failed connecting subscriber");
        subscriber
            .set_subscribe(b"frame")
            .expect("failed subscribing");

        // 1. The **winit::EventsLoop** for handling events.
        let event_loop = glium::glutin::event_loop::EventLoop::new();
        // 2. Parameters for building the Window.
        let wb = glium::glutin::window::WindowBuilder::new()
            .with_inner_size(glium::glutin::dpi::LogicalSize::new(1024.0, 768.0))
            .with_title("Hello world");
        // 3. Parameters for building the OpenGL context.
        let cb = glium::glutin::ContextBuilder::new();
        // 4. Build the Display with the given window and OpenGL context parameters and register the
        //    window with the events_loop.
        let display = glium::Display::new(wb, cb, &event_loop).unwrap();

        implement_vertex!(SnowVertex, position, tex_coords);

        let data = vec![
            SnowVertex {
                position: [-1.0, -1.0],
                tex_coords: [0.0, 1.0],
            },
            SnowVertex {
                position: [-1.0, 1.0],
                tex_coords: [0.0, 0.0],
            },
            SnowVertex {
                position: [1.0, 1.0],
                tex_coords: [1.0, 0.0],
            },
            SnowVertex {
                position: [1.0, -1.0],
                tex_coords: [1.0, 1.0],
            },
            SnowVertex {
                position: [-1.0, -1.0],
                tex_coords: [0.0, 1.0],
            },
            SnowVertex {
                position: [1.0, 1.0],
                tex_coords: [1.0, 0.0],
            },
        ];

        let _indices: Vec<u8> = vec![0, 1, 2, 3, 0, 2];
        let vertex_src = r#"
    #version 140

    in vec2 position;
in vec2 tex_coords;
out vec2 v_tex_coords;

    uniform mat4 matrix;

    void main() {
        v_tex_coords = tex_coords;
        gl_Position = matrix * vec4(position, 0.0, 1.0);
    }
"#;
        let fragment_src = r#"
    #version 330 core
   in vec2 v_tex_coords;
   out vec4 color;
  
   uniform sampler2D tex;
  
   void main()
   {
      vec4 pre = texture(tex, v_tex_coords);
      color.rgba = pre.bgra;
  }"#;
        let program =
            glium::Program::from_source(&display, vertex_src, fragment_src, None).unwrap();

        use glium::Surface;

        event_loop.run(move |ev, _, control_flow| {
            let next_frame_time =
                std::time::Instant::now() + std::time::Duration::from_nanos(16_666_667);
            *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);
            match ev {
                glutin::event::Event::WindowEvent { event, .. } => match event {
                    glutin::event::WindowEvent::CloseRequested => {
                        *control_flow = glutin::event_loop::ControlFlow::Exit;
                        return;
                    }

                    _ => return,
                },

                glutin::event::Event::DeviceEvent { event, .. } => match event {
                    glutin::event::DeviceEvent::Key(input, ..) => {
                        println!("[{}], {}", input.scancode, input.scancode as u8 as char);
                        return;
                    }
                    _ => return,
                },

                _ => (),
            }

            let mut frame = display.draw();
            frame.clear_color(0.0, 0.0, 1.0, 1.0);

            let _envelope = subscriber
                .recv_string(0)
                .expect("failed receiving envelope")
                .unwrap();
            let width =
                bincode::deserialize(&subscriber.recv_bytes(0).expect("failed receiving message"))
                    .unwrap();

            let height: u32 =
                bincode::deserialize(&subscriber.recv_bytes(0).expect("failed receiving message"))
                    .unwrap();

            let message = subscriber.recv_bytes(0).expect("failed receiving message");

            let image = glium::texture::RawImage2d::from_raw_rgba(message, (width, height));
            let texture = glium::texture::SrgbTexture2d::new(&display, image).unwrap();

            let (our_width, our_height) = frame.get_dimensions();
            let scale_x; // = 1.0;
            let scale_y; // = 1.0 / (width as f32 / height as f32);

            scale_x = width as f32 / our_width as f32;
            scale_y = height as f32 / our_height as f32;

            let uniforms = uniform! {

            matrix: [
                [scale_x, 0.0, 0.0, 0.0],
                [0.0, scale_y, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ 0.0 , 0.0, 0.0, 1.0f32],
            ],
                tex: &texture,
            };

            frame
                .draw(
                    &glium::vertex::VertexBuffer::new(&display, &data).unwrap(),
                    &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                    &program,
                    &uniforms,
                    &Default::default(),
                )
                .unwrap();

            frame.finish().unwrap();
        });
    }
}
