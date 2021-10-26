use std::env;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::u64;
use zmq::SNDMORE;

mod networking;
mod recording;

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

    //    let w_frame = context.socket(zmq::PUB).unwrap();
    //    w_frame
    //        .bind(&format!("tcp://*:{}", networking::HOST_FRAME_STREAM_PORT))
    //        .expect("Failed binding out socket for host");

    // A token to allow us to identify which event is for the `UdpSocket`.
    const UDP_SOCKET: Token = Token(0);

    // Create a poll instance.
    let mut poll = Poll::new().unwrap();

    // Setup the UDP socket.
    let addr = format!("127.0.0.1:{}", networking::HOST_FRAME_STREAM_PORT)
        .parse()
        .unwrap();
    let mut subscribers: Vec<std::net::SocketAddr> = vec![];

    subscribers.push(
        format!("127.0.0.1:{}", networking::CLIENT_FRAME_STREAM_PORT)
            .parse()
            .unwrap(),
    );

    let mut socket = UdpSocket::bind(addr).unwrap();

    // Register our socket with the token defined above and an interest in being
    // `READABLE`.
    poll.registry()
        .register(&mut socket, UDP_SOCKET, Interest::WRITABLE)
        .unwrap();

    let mut fc = 0;

    loop {
        let xid = u64::from_str_radix(&args[2], 16).unwrap();
        let image = recording::record_linux(display, xid);
        {
            let packet = image.data.as_ref().unwrap().to_vec();

            for s in &subscribers {
                socket.send_to(&packet, *s).unwrap();
                fc += 1;
                println!("sent frame {} to: {}", fc, *s);
            }

            /*
            w_frame
                .send("frame", zmq::SNDMORE)
                .expect("failed sending frame envelope");
            w_frame
                .send(bincode::serialize(&image.width).unwrap(), zmq::SNDMORE)
                .expect("failed sending frame width");
            w_frame
                .send(bincode::serialize(&image.height).unwrap(), zmq::SNDMORE)
                .expect("failed sending frame height");
            w_frame
                .send(image.data.unwrap(), 0)
                .expect("failed sending frame");
                */
        }

        thread::sleep(Duration::from_millis(1));
    }
}

use mio::net::UdpSocket;
use mio::{Events, Interest, Poll, Token};
use std::io;

fn host_udp() -> io::Result<()> {
    // Our event loop.
    loop {}
}

#[cfg(target_os = "linux")]
fn handle_input(context: &zmq::Context) {
    use sweetacid_evdev::uinput::VirtualDeviceBuilder;
    use sweetacid_evdev::AbsoluteAxisType as Axis;
    use sweetacid_evdev::AttributeSet;
    use sweetacid_evdev::InputId;
    use sweetacid_evdev::Key;

    use input_event_codes as e;

    let mut keys = vec![];
    keys.push(Key::new(e::BTN_A));
    keys.push(Key::new(e::BTN_B));
    keys.push(Key::new(e::BTN_X));
    keys.push(Key::new(e::BTN_Y));
    keys.push(Key::new(e::BTN_TL));
    keys.push(Key::new(e::BTN_TR));
    keys.push(Key::new(e::BTN_TL2));
    keys.push(Key::new(e::BTN_TR2));
    keys.push(Key::new(e::BTN_SELECT));
    keys.push(Key::new(e::BTN_START));
    keys.push(Key::new(e::BTN_THUMBL));
    keys.push(Key::new(e::BTN_THUMBR));

    let mut key_attribs: AttributeSet<Key> = AttributeSet::new();
    for k in &keys {
        key_attribs.insert(*k);
    }

    let mut axes: AttributeSet<Axis> = AttributeSet::new();
    axes.insert(Axis::ABS_X);
    axes.insert(Axis::ABS_Y);
    axes.insert(Axis::ABS_Z);
    axes.insert(Axis::ABS_RX);
    axes.insert(Axis::ABS_RY);
    axes.insert(Axis::ABS_RZ);

    let mut vd = VirtualDeviceBuilder::new()
        .unwrap()
        .name("Rusty Snow Virtual Gamepad")
        .input_id(InputId::new(
            sweetacid_evdev::BusType::BUS_USB,
            0x045e,
            0x028e,
            2,
        ))
        .with_keys(&key_attribs)
        .unwrap()
        .with_absolute_axes(&axes)
        .unwrap()
        .build()
        .unwrap();

    thread::sleep(Duration::from_millis(1500));

    /*
        use evdev_rs::enums;
        //use evdev_rs::enums::EventCode::EV_KEY;
        use evdev_rs::enums::EventType::EV_KEY;
        use evdev_rs::enums::EventType::EV_SYN;
        use evdev_rs::{Device, InputEvent, TimeVal, UInputDevice, UninitDevice};

        let fd = std::fs::File::open(
            "/dev/input/by-id/usb-Ducky_Ducky_One2_SF_RGB_DK-V1.06-200925-event-kbd",
        )
        .unwrap();
        let uninit_device = UninitDevice::new().unwrap();
        let device = uninit_device.set_file(fd).unwrap();
        let ui = UInputDevice::create_from_device(&device).unwrap();
    */
    let r_input = context.socket(zmq::PULL).unwrap();
    assert!(r_input
        .bind(&format!("tcp://*:{}", networking::HOST_INPUT_STREAM_PORT))
        .is_ok());

    loop {
        let identity = r_input.recv_string(0).unwrap().unwrap();
        let envelope = r_input.recv_string(0).unwrap().unwrap();
        // bincode::deserialize::<Vec<u32>>(&r_input.recv_bytes(0).unwrap())

        println!("[H] [{}] envelope: {}", identity, envelope); // Envelope

        let key_str = r_input.recv_string(0).unwrap().unwrap();
        let state_str = r_input.recv_string(0).unwrap().unwrap();

        println!("[H] [{}] {} {:?}", identity, state_str, key_str);

        let key = u16::from_str_radix(&key_str, 10).unwrap();
        let state = i32::from_str_radix(&state_str, 10).unwrap();

        if key == 17 {
            let mut value = 127;
            if state == 1 {
                value = 255;
            }

            vd.emit(&[sweetacid_evdev::InputEvent::new(
                sweetacid_evdev::EventType::ABSOLUTE,
                0x01,
                value,
            )])
            .unwrap();
        } else if key == 31 {
            let mut value = 127;
            if state == 1 {
                value = 0;
            }

            vd.emit(&[sweetacid_evdev::InputEvent::new(
                sweetacid_evdev::EventType::ABSOLUTE,
                0x01,
                value,
            )])
            .unwrap();
        } else if key == 30 {
            let mut value = 127;
            if state == 1 {
                value = 0;
            }

            vd.emit(&[sweetacid_evdev::InputEvent::new(
                sweetacid_evdev::EventType::ABSOLUTE,
                0x00,
                value,
            )])
            .unwrap();
        } else if key == 32 {
            let mut value = 127;
            if state == 1 {
                value = 255;
            }

            vd.emit(&[sweetacid_evdev::InputEvent::new(
                sweetacid_evdev::EventType::ABSOLUTE,
                0x00,
                value,
            )])
            .unwrap();
        } else {
            vd.emit(&[sweetacid_evdev::InputEvent::new(
                sweetacid_evdev::EventType::KEY,
                e::BTN_SOUTH,
                state,
            )])
            .unwrap();
        }

        /*
        ui.write_event(&InputEvent::new(
            &TimeVal::new(0, 0),
            //&enums::EventCode::from_str(&EV_KEY, "KEY_A").unwrap(),
            &enums::EventCode::EV_UNK {
                event_type: 1,
                event_code: key,
            },
            state,
        ))
        .unwrap();

        ui.write_event(&InputEvent::new(
            &TimeVal::new(0, 0),
            &enums::EventCode::from_str(&EV_SYN, "SYN_REPORT").unwrap(),
            state,
        ))
        .unwrap();

        */
        //thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(target_os = "linux")]
fn host(args: Vec<String>) {
    /*
    let zzzz = input_linux::uinput::UInputHandle::new(uinput);

    let name = bincode::serialize("Rusty Snow Virtual Keyboard").unwrap();
    let mut sigh = vec![];
    for b in name {
        sigh.push(b as i8);
    }
    use std::convert::TryInto;
    let uinput_setup = input_linux_sys::uinput_setup {
        id: input_linux_sys::input_id {
            bustype: 0x03,
            vendor: 0x02,
            product: 0x03,
            version: 2,
        },
        name: sigh[..].try_into().unwrap(),
        ff_effects_max: 0,
    };

    zzzz.dev_setup(uinput_setup);

    let fd = std::fs::File::open("/dev/input/event74");

    */

    let context = zmq::Context::new();

    {
        let ctx = context.clone();
        thread::spawn(move || handshake(&ctx));
    }
    {
        let ctx = context.clone();
        thread::spawn(move || send_frames(&ctx));
    }
    {
        let ctx = context.clone();
        thread::spawn(move || handle_input(&ctx));
    }

    loop {}
}

#[cfg(target_os = "linux")]
fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|x| format!("{:02x}", x))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(target_os = "linux")]
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
        host(args);
    } else {
        let mut client = networking::Client::new();
        client.connect(String::from("192.168.0.103"), String::from("tony"));

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

    let context = &client.context;
    let w_input = context.socket(zmq::PUSH).unwrap();
    w_input
        .connect(&format!(
            "tcp://{}:{}",
            "localhost",
            networking::HOST_INPUT_STREAM_PORT
        ))
        .expect(&format!(
            "[C] [{}] failed connecting to host at {}",
            client.user_id,
            format!(
                "tcp://{}:{}",
                "localhost",
                networking::HOST_INPUT_STREAM_PORT
            )
        ));

    let mtx0 = std::sync::Mutex::new(SnowFrame {
        data: None,
        width: 0,
        height: 0,
    });
    let mtx1 = std::sync::Mutex::new(SnowFrame {
        data: None,
        width: 0,
        height: 0,
    });

    static mut RENDERER: Renderer = Renderer::new();

    thread::spawn(move || {
        let mut facade = NetFacade::new();

        loop {
            unsafe {
                let frame = facade.get_frame();

                RENDERER.write(frame);
                RENDERER.swap_buffers();
            }

            /*
            let mut guard = mtx0.lock().unwrap();
            let frame = facade.get_frame();

            guard.data = frame.data;
            guard.width = frame.width;
            guard.height = frame.height;
            */
        }
    });

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

                    w_input.send(&client.user_id, SNDMORE).unwrap();
                    w_input.send("KEY", SNDMORE).unwrap();
                    w_input.send(&format!("{}", input.scancode), 0).unwrap();

                    use glutin::event::ElementState;

                    let state = match input.state {
                        ElementState::Pressed => 1,
                        ElementState::Released => 0,
                    };
                    w_input.send(&format!("{:?}", state), 0).unwrap();
                    return;
                }

                _ => return,
            },

            _ => (),
        }

        /*
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
        */

        unsafe {
            let frame = RENDERER.read();
            draw(
                &display,
                frame.width,
                frame.height,
                &program,
                frame
                    .data
                    .as_ref()
                    .unwrap_or(&vec![0; (frame.width * frame.height * 4) as usize])
                    .to_vec(),
            );
        }
        /*
                    frame_stream
                    .recv_bytes(0)
                    .expect("failed receiving message");
        */
        /*
        if let Err(e) = socket.recv_from(&mut buf) {
            if e.kind() != io::ErrorKind::WouldBlock {
                println!("{}", e);
            }
            break;
        } else if let Ok((size, source)) = socket.recv_from(&mut buf) {
            println!("packet size: {}", size);
            width = bincode::deserialize::<u32>(&buf[0..31]).unwrap();
            height = bincode::deserialize::<u32>(&buf[31..63]).unwrap();
            message = buf[63..size].to_vec();
        }
        */

        /*
        match event.token() {
            UDP_SOCKET => loop {
                // In this loop we receive all packets queued for the socket.
                match socket.recv_from(&mut buf) {
                    Ok((packet_size, source_address)) => {
                        println!("packet size: {}", packet_size);
                        width = bincode::deserialize::<u32>(&buf[0..31]).unwrap();
                        height = bincode::deserialize::<u32>(&buf[31..63]).unwrap();
                        message = buf[63..packet_size].to_vec();
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        // If we get a `WouldBlock` error we know our socket
                        // has no more packets queued, so we can return to
                        // polling and wait for some more.
                        break;
                    }
                    Err(e) => {
                        // If it was any other kind of error, something went
                        // wrong and we terminate with an error.
                        println!("{}", e);
                    }
                }
            },
            _ => {
                // This should never happen as we only registered our
                // `UdpSocket` using the `UDP_SOCKET` token, but if it ever
                // does we'll log it.
                println!("Got event for unexpected token: {:?}", event);
            }
        }
        */
    });
}

struct Renderer {
    buffers: [SnowFrame; 2],
    read_from: usize,
    write_to: usize,
}

impl Renderer {
    const fn new() -> Self {
        Self {
            buffers: [
                SnowFrame {
                    data: None,
                    width: 0,
                    height: 0,
                },
                SnowFrame {
                    data: None,
                    width: 0,
                    height: 0,
                },
            ],
            read_from: 0,
            write_to: 1,
        }
    }

    fn read(&self) -> &SnowFrame {
        &self.buffers[self.read_from]
    }

    fn write(&mut self, frame: SnowFrame) {
        self.buffers[self.write_to] = frame;
    }

    fn swap_buffers(&mut self) {
        //  TODO: schedule swap buffers, don't swap them directly
        self.read_from = (self.read_from + 1) % 2;
        self.write_to = (self.write_to + 1) % 2;
    }
}

// A token to allow us to identify which event is for the `UdpSocket`.
const UDP_SOCKET: Token = Token(0);

struct NetFacade {
    events: Events,
    poll: Poll,
    socket: UdpSocket,
    buf: [u8; 1 << 16],
}

impl NetFacade {
    fn new() -> Self {
        // Create storage for events. Since we will only register a single socket, a
        // capacity of 1 will do.
        let mut events = Events::with_capacity(1);

        // Create a poll instance.
        let mut poll = Poll::new().unwrap();

        // Setup the UDP socket.
        let addr = format!("127.0.0.1:{}", networking::CLIENT_FRAME_STREAM_PORT)
            .parse()
            .unwrap();

        println!("You can connect to the server using `nc`:");
        println!(" $ nc -u 127.0.0.1 9000");
        println!("Anything you type will be echoed back to you.");

        let mut socket = UdpSocket::bind(addr).unwrap();

        // Register our socket with the token defined above and an interest in being
        // `READABLE`.
        poll.registry()
            .register(&mut socket, UDP_SOCKET, Interest::READABLE)
            .unwrap();

        socket
            .connect(
                format!("127.0.0.1:{}", networking::HOST_FRAME_STREAM_PORT)
                    .parse()
                    .unwrap(),
            )
            .unwrap();

        // Initialize a buffer for the UDP packet. We use the maximum size of a UDP
        // packet, which is the maximum value of 16 a bit integer.
        let mut buf = [0; 1 << 16];

        return Self {
            events,
            poll,
            socket,
            buf,
        };
    }

    fn get_frame(&mut self) -> SnowFrame {
        // Poll to check if we have events waiting for us.
        self.poll.poll(&mut self.events, None).unwrap();

        // Process each event.
        for event in self.events.iter() {
            // Validate the token we registered our socket with,
            // in this example it will only ever be one but we
            // make sure it's valid none the less.
            match event.token() {
                UDP_SOCKET => {
                    loop {
                        match self.socket.recv_from(&mut self.buf) {
                            Ok((packet_size, source_address)) => {
                                let message = self.buf.to_vec();

                                let d = mozjpeg::Decompress::with_markers(mozjpeg::NO_MARKERS)
                                    .from_mem(&message)
                                    .unwrap();

                                assert!(d.color_space() == mozjpeg::ColorSpace::JCS_YCbCr);

                                let mut rgb = d.rgba().unwrap();
                                assert!(rgb.color_space() == mozjpeg::ColorSpace::JCS_EXT_RGBA);

                                let width = rgb.width() as u32;
                                let height = rgb.height() as u32;

                                let mut pixels: Vec<u32> = rgb.read_scanlines().unwrap();
                                assert!(rgb.finish_decompress());

                                let mut u8slice = bincode::serialize(&pixels).unwrap();
                                for i in 0..8 {
                                    u8slice.pop();
                                }

                                return SnowFrame {
                                    data: Some(u8slice),
                                    width,
                                    height,
                                };
                            }
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                // If we get a `WouldBlock` error we know our socket
                                // has no more packets queued, so we can return to
                                // polling and wait for some more.

                                println!("{}", e);
                                return SnowFrame {
                                    data: None,
                                    width: 0,
                                    height: 0,
                                };
                            }
                            Err(e) => {
                                // If it was any other kind of error, something went
                                // wrong and we terminate with an error.
                                println!("{}", e);
                                return SnowFrame {
                                    data: None,
                                    width: 0,
                                    height: 0,
                                };
                            }
                        }
                    }
                }

                _ => {
                    println!("got token: {:?}", event);
                    return SnowFrame {
                        data: None,
                        width: 0,
                        height: 0,
                    };
                }
            }
        }

        return SnowFrame {
            data: None,
            width: 0,
            height: 0,
        };
    }
}

#[derive(Clone)]
struct SnowFrame {
    data: Option<Vec<u8>>,
    width: u32,
    height: u32,
}

fn draw(
    display: &glium::Display,
    width: u32,
    height: u32,
    program: &glium::Program,
    image: Vec<u8>,
) {
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

    use glium::Surface;
    let mut frame = display.draw();
    frame.clear_color(0.0, 0.0, 0.0, 1.0);

    let image = glium::texture::RawImage2d::from_raw_rgba(image, (width, height));
    let texture = glium::texture::SrgbTexture2d::new(display, image).unwrap();

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
            &glium::vertex::VertexBuffer::new(display, &data).unwrap(),
            &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
            &program,
            &uniforms,
            &Default::default(),
        )
        .unwrap();

    frame.finish().unwrap();
}

fn yes() {
    use gilrs::{Button, Event, Gilrs};

    let mut gilrs = Gilrs::new().unwrap();

    let mut active_gamepad = None;
    // Iterate over all connected gamepads
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    use sweetacid_evdev::uinput::VirtualDeviceBuilder;
    use sweetacid_evdev::AbsoluteAxisType as Axis;
    use sweetacid_evdev::AttributeSet;
    use sweetacid_evdev::InputId;
    use sweetacid_evdev::Key;

    use input_event_codes as e;

    let mut keys = vec![];
    keys.push(Key::new(e::BTN_A));
    keys.push(Key::new(e::BTN_B));
    keys.push(Key::new(e::BTN_X));
    keys.push(Key::new(e::BTN_Y));
    keys.push(Key::new(e::BTN_TL));
    keys.push(Key::new(e::BTN_TR));
    keys.push(Key::new(e::BTN_TL2));
    keys.push(Key::new(e::BTN_TR2));
    keys.push(Key::new(e::BTN_SELECT));
    keys.push(Key::new(e::BTN_START));
    keys.push(Key::new(e::BTN_THUMBL));
    keys.push(Key::new(e::BTN_THUMBR));

    let mut key_attribs: AttributeSet<Key> = AttributeSet::new();
    for k in &keys {
        key_attribs.insert(*k);
    }

    let mut axes: AttributeSet<Axis> = AttributeSet::new();
    axes.insert(Axis::ABS_X);
    axes.insert(Axis::ABS_Y);
    axes.insert(Axis::ABS_Z);
    axes.insert(Axis::ABS_RX);
    axes.insert(Axis::ABS_RY);
    axes.insert(Axis::ABS_RZ);

    let mut vd = VirtualDeviceBuilder::new()
        .unwrap()
        .name("Rusty Snow Virtual Gamepad")
        .input_id(InputId::new(
            sweetacid_evdev::BusType::BUS_USB,
            0x045e,
            0x028e,
            2,
        ))
        .with_keys(&key_attribs)
        .unwrap()
        .with_absolute_axes(&axes)
        .unwrap()
        .build()
        .unwrap();

    thread::sleep(Duration::from_millis(1500));

    loop {
        /*
                    for k in &keys {
                        vd.emit(&[evdev::InputEvent::new(
                            evdev::EventType::KEY,
                            (*k).code(),
                            1,
                        )])
                        .unwrap();

                        thread::sleep(Duration::from_millis(500));

                        vd.emit(&[evdev::InputEvent::new(
                            evdev::EventType::KEY,
                            (*k).code(),
                            1,
                        )])
                        .unwrap();

                        thread::sleep(Duration::from_millis(500));
                    }
        */

        let mut to_emit: Vec<sweetacid_evdev::InputEvent> = vec![];

        // Examine new events
        while let Some(Event { id, event, time }) = gilrs.next_event() {
            //println!("{:?} New event from {}: {:?}", time, id, event);
            active_gamepad = Some(id);

            use gilrs::ev::EventType::{AxisChanged, ButtonChanged};

            match event {
                AxisChanged(axis, mut value, code) => {
                    let orig = value;
                    println!("{:?} | {} | {}", axis, orig, code);

                    //if axis != gilrs::Axis::LeftStickY {
                    //  value is [-1 - 1]
                    //  convert to [0 - 2] -> [0 - 200] -> [0 - 255] ranges
                    let deadzone = 0.05;
                    if value.abs() <= deadzone {
                        value = 0.0;
                    }

                    value += 1.0;

                    if value > 0.0 {
                        println!("after deadzone: {}", value);
                    }

                    let actual = lerp(0.0, 255.0, (value * 100.0) / 200.0);

                    fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
                        return v0 + t * (v1 - v0);
                    }

                    println!("orig: {} | we get {}", value, actual as i32);

                    to_emit.push(sweetacid_evdev::InputEvent::new(
                        sweetacid_evdev::EventType::ABSOLUTE,
                        code.into_u32() as u16,
                        actual as i32,
                    ))
                    // }
                }

                ButtonChanged(_, state, code) => to_emit.push(sweetacid_evdev::InputEvent::new(
                    sweetacid_evdev::EventType::KEY,
                    code.into_u32() as u16,
                    state as i32,
                )),
                _ => {
                    println!("rip {:?}", event);
                }
            };
        }

        vd.emit(&to_emit).unwrap();

        to_emit.clear();
        // You can also use cached gamepad state
        /*
        if let Some(gamepad) = active_gamepad.map(|id| gilrs.gamepad(id)) {
            let axis = gilrs::Axis::LeftStickX;
            let mut value = gamepad.value(axis);
            let code = 0x00;
            let orig = value;
            println!("{:?} | {}", axis, orig);

            //if axis != gilrs::Axis::LeftStickY {
            //  value is [-1 - 1]
            //  convert to [0 - 2] -> [0 - 200] -> [0 - 255] ranges
            let deadzone = 0.05;
            if value.abs() <= deadzone {
                value = 0.0;
            }

            value += 1.0;

            if value > 0.0 {
                println!("after deadzone: {}", value);
            }

            let actual = lerp(0.0, 255.0, (value * 100.0) / 200.0);

            fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
                return v0 + t * (v1 - v0);
            }

            println!("orig: {} | we get {}", value, actual as i32);

            to_emit.push(sweetacid_evdev::InputEvent::new(
                sweetacid_evdev::EventType::ABSOLUTE,
                code as u16,
                actual as i32,
            ));

            if gamepad.is_pressed(Button::South) {
                println!("Button South is pressed (XBox - A, PS - X)");
            }
        }
        */
    }
}
