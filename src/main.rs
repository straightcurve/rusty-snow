mod networking;
mod recording;

use std::env;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::u64;
use zmq::SNDMORE;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate glium;

#[derive(Copy, Clone)]
struct SnowVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

#[cfg(target_os = "windows")]
fn host(args: Vec<String>) {}

#[cfg(target_os = "linux")]
fn handshake(context: &zmq::Context) {
    let rw_primary = context.socket(zmq::ROUTER).unwrap();
    assert!(rw_primary
        .bind(&format!("tcp://*:{}", networking::HOST_PRIMARY_PORT))
        .is_ok());

    loop {
        {
            let identity = rw_primary.recv_string(0).unwrap().unwrap();

            //  read empty string
            rw_primary.recv_string(0).unwrap().unwrap();

            let envelope = rw_primary.recv_string(0).unwrap().unwrap();
            println!("[H] [{}] envelope: {}", identity, envelope); // Envelope

            if envelope == "SYN" {
                rw_primary.send(&identity, SNDMORE).unwrap();
                rw_primary.send("", SNDMORE).unwrap();
                rw_primary.send("ACK", 0).unwrap();
                continue;
            } else if envelope == "NAME" {
                rw_primary.send(&identity, SNDMORE).unwrap();
                rw_primary.send("", SNDMORE).unwrap();
                rw_primary.send("NAME_OK", 0).unwrap();
                continue;
            } else if envelope == "input" {
                println!(
                    "[H] [{}] pressed {:?}",
                    identity,
                    bincode::deserialize::<Vec<u32>>(&rw_primary.recv_bytes(0).unwrap())
                );
            }
        }

        // Encourage workers until it's time to fire them
        /*
        if start_time.elapsed() < allowed_duration {
            broker.send(&identity, SNDMORE).unwrap();
            broker.send("", SNDMORE).unwrap();
            broker
                .send(&format!("Work harder, {}", identity), 0)
                .unwrap();
        } else {
            broker.send(&identity, SNDMORE).unwrap();
            broker.send("DC", 0).unwrap();
            workers_fired += 1;
            if workers_fired >= worker_pool_size {
                break;
            }
            }
        */
    }
}

#[cfg(target_os = "linux")]
fn send_frames(context: &zmq::Context) {
    let args: Vec<String> = env::args().collect();
    let display = recording::open_display();
    let w_frame = context.socket(zmq::PUB).unwrap();
    w_frame
        .bind(&format!("tcp://*:{}", networking::HOST_FRAME_STREAM_PORT))
        .expect("Failed binding out socket for host");

    loop {
        let xid = u64::from_str_radix(&args[2], 16).unwrap();
        let mut image = recording::record_linux(display, xid);
        {
            w_frame
                .send("frame", zmq::SNDMORE)
                .expect("failed sending frame envelope");
            w_frame
                .send(bincode::serialize(&image.width).unwrap(), zmq::SNDMORE)
                .expect("failed sending frame width");
            w_frame
                .send(bincode::serialize(&image.height).unwrap(), zmq::SNDMORE)
                .expect("failed sending frame height");

            w_frame.send(image.data, 0).expect("failed sending frame");
        }
        image.free();
        thread::sleep(Duration::from_millis(16));
    }
}

#[cfg(target_os = "linux")]
fn host(args: Vec<String>) {
    let context = zmq::Context::new();

    {
        let ctx = context.clone();
        thread::spawn(move || handshake(&ctx));
    }
    {
        let ctx = context.clone();
        thread::spawn(move || send_frames(&ctx));
    }

    loop {}
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|x| format!("{:02x}", x))
        .collect::<Vec<_>>()
        .join("")
}

fn worker_task(id: String) {
    let mut client = networking::Client::new();
    client.connect(String::from("localhost"), id);

    let worker = &client.rw_primary;
    worker.send("NAME", SNDMORE).unwrap();
    worker.send(&client.user_id, 0).unwrap();

    let mut name_ok = false;
    while !name_ok {
        let envelope = worker
            .recv_string(0)
            .expect(&format!(
                "[C] [{}] failed reading envelope 1",
                client.user_id
            ))
            .expect(&format!(
                "[C] [{}] failed reading envelope 2",
                client.user_id
            ));

        println!("[C] [{}] envelope: {}", client.user_id, envelope); // Envelope

        if envelope.eq("DC") {
            println!("[C] [{}] disconnected by server", client.user_id);
            break;
        } else if envelope == "NAME_OK" {
            name_ok = true;
        }
    }

    do_client_stuff(client);
    /*
    loop {

                let mut input: Vec<u32> = vec![];
                input.push(32);
                input.push(47);
                input.push(125);

                worker.send("input", SNDMORE).unwrap();
                worker.send(bincode::serialize(&input).unwrap(), 0).unwrap();
    }
        */
}

#[cfg(target_os = "linux")]
fn test_bench() {
    let worker_pool_size = 1;
    let allowed_duration = Duration::new(5, 0);
    let host = networking::Host::new();
    let broker = host.rw_primary;

    let mut thread_pool = Vec::new();
    /*
    for c in 0..worker_pool_size {
        let child = thread::spawn(move || {
            worker_task(format!("Client_{}", c), true);
        });
        thread_pool.push(child);
    }

    */

    thread_pool.push(thread::spawn(move || {
        worker_task(String::from("acid"));
    }));
    /*
    thread_pool.push(thread::spawn(move || {
        worker_task(String::from("client 1"));
    }));
    thread_pool.push(thread::spawn(move || {
        worker_task(String::from("client 2"));
    }));

    */
    let start_time = Instant::now();
    let mut workers_fired = 0;
    loop {
        // Next message gives us least recently used worker
        let identity = broker.recv_string(0).unwrap().unwrap();

        //  read empty string
        broker.recv_string(0).unwrap().unwrap();

        let envelope = broker.recv_string(0).unwrap().unwrap();
        println!("[H] [{}] envelope: {}", identity, envelope); // Envelope

        if envelope == "SYN" {
            broker.send(&identity, SNDMORE).unwrap();
            broker.send("", SNDMORE).unwrap();
            broker.send("ACK", 0).unwrap();
            continue;
        } else if envelope == "NAME" {
            broker.send(&identity, SNDMORE).unwrap();
            broker.send("", SNDMORE).unwrap();
            broker.send("NAME_OK", 0).unwrap();
            continue;
        } else if envelope == "input" {
            println!(
                "[H] [{}] pressed {:?}",
                identity,
                bincode::deserialize::<Vec<u32>>(&broker.recv_bytes(0).unwrap())
            );
        }

        // Encourage workers until it's time to fire them
        if start_time.elapsed() < allowed_duration {
            broker.send(&identity, SNDMORE).unwrap();
            broker.send("", SNDMORE).unwrap();
            broker
                .send(&format!("Work harder, {}", identity), 0)
                .unwrap();
        } else {
            broker.send(&identity, SNDMORE).unwrap();
            broker.send("DC", 0).unwrap();
            workers_fired += 1;
            if workers_fired >= worker_pool_size {
                break;
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    if args.len() != 1 && args[1] == "host" {
        //test_bench();
        host(args);
    } else {
        let mut client = networking::Client::new();
        client.connect(String::from("localhost"), String::from("tony"));

        let worker = &client.rw_primary;
        worker.send("NAME", SNDMORE).unwrap();
        worker.send(&client.user_id, 0).unwrap();

        let mut name_ok = false;
        while !name_ok {
            let envelope = worker
                .recv_string(0)
                .expect(&format!(
                    "[C] [{}] failed reading envelope 1",
                    client.user_id
                ))
                .expect(&format!(
                    "[C] [{}] failed reading envelope 2",
                    client.user_id
                ));

            println!("[C] [{}] envelope: {}", client.user_id, envelope); // Envelope

            if envelope.eq("DC") {
                println!("[C] [{}] disconnected by server", client.user_id);
                break;
            } else if envelope == "NAME_OK" {
                name_ok = true;
            }
        }

        do_client_stuff(client);
    }
}

fn do_client_stuff(client: networking::Client) {
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
    let program = glium::Program::from_source(&display, vertex_src, fragment_src, None).unwrap();

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
                    // scancode: u32
                    println!("glutin: keyboard: [{}]", input.scancode);
                    return;
                }

                _ => return,
            },

            _ => (),
        }

        let mut frame = display.draw();
        frame.clear_color(0.0, 0.0, 0.0, 1.0);

        let frame_stream = &client.r_frame;

        let _envelope = frame_stream
            .recv_string(0)
            .expect("failed receiving envelope")
            .unwrap();
        let width = bincode::deserialize(
            &frame_stream
                .recv_bytes(0)
                .expect("failed receiving message"),
        )
        .unwrap();

        let height: u32 = bincode::deserialize(
            &frame_stream
                .recv_bytes(0)
                .expect("failed receiving message"),
        )
        .unwrap();

        let message = frame_stream
            .recv_bytes(0)
            .expect("failed receiving message");

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
