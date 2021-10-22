use std::thread;
use std::time::Duration;

pub const HOST_PRIMARY_PORT: u32 = 5564;
pub const HOST_FRAME_STREAM_PORT: u32 = 5565;

pub struct Host {
    pub context: zmq::Context,

    /**
     * used for reading/writing general data
     *
     * e.g. handling new connections
     */
    pub rw_primary: zmq::Socket,

    /**
     * used for writing images
     */
    pub w_frame: zmq::Socket,
}

impl Host {
    pub fn new() -> Self {
        let context = zmq::Context::new();
        let w_frame = context.socket(zmq::PUB).unwrap();
        w_frame
            .bind(&format!("tcp://*:{}", HOST_FRAME_STREAM_PORT))
            .expect("Failed binding out socket for host");

        let rw_primary = context.socket(zmq::ROUTER).unwrap();
        assert!(rw_primary
            .bind(&format!("tcp://*:{}", HOST_PRIMARY_PORT))
            .is_ok());

        Self {
            context,
            w_frame,
            rw_primary,
        }
    }

    pub fn send<T>(&self, data: T, flags: i32) -> Result<(), zmq::Error>
    where
        T: zmq::Sendable,
    {
        self.w_frame.send(data, flags)
    }

    pub fn send_frame(&self, width: u32, height: u32, data: &[u8]) {
        self.send("frame", zmq::SNDMORE)
            .expect("failed sending frame envelope");
        self.send(bincode::serialize(&width).unwrap(), zmq::SNDMORE)
            .expect("failed sending frame width");
        self.send(bincode::serialize(&height).unwrap(), zmq::SNDMORE)
            .expect("failed sending frame height");

        self.send(data, 0).expect("failed sending frame");
    }
}

pub struct Client {
    pub user_id: String,

    /**
     * used for connecting and
     * sending general data
     *
     * e.g. user id
     */
    pub rw_primary: zmq::Socket,

    /**
     * used for reading images
     */
    pub r_frame: zmq::Socket,

    context: zmq::Context,
    input: zmq::Socket,
}

impl Client {
    pub fn new() -> Self {
        let context = zmq::Context::new();
        let input = context.socket(zmq::PUB).unwrap();

        //input
        //    .bind("tcp://*:5564")
        //    .expect("Failed binding input socket for client");

        let rw_primary = context.socket(zmq::REQ).unwrap();
        let r_frame = context.socket(zmq::SUB).unwrap();

        Self {
            context,
            input,
            rw_primary,
            r_frame,
            user_id: String::from(""),
        }
    }

    pub fn connect(&mut self, url: String, socket_id: String) {
        self.user_id = socket_id;

        let identity = bincode::serialize(&self.user_id).unwrap();
        self.rw_primary.set_identity(&identity).unwrap();

        self.rw_primary
            .connect(&format!("tcp://{}:{}", url, HOST_PRIMARY_PORT))
            .expect(&format!(
                "[C] [{}] failed connecting to host at {}",
                self.user_id,
                format!("tcp://{}:{}", url, HOST_PRIMARY_PORT)
            ));

        self.rw_primary.send("SYN", 0).unwrap();

        let envelope = self
            .rw_primary
            .recv_string(0)
            .expect(&format!("[C] [{}] failed reading envelope 1", self.user_id))
            .expect(&format!("[C] [{}] failed reading envelope 2", self.user_id));

        if envelope != "ACK" {
            panic!(
                "[C] [{}] failed connecting to host at {}",
                self.user_id, url
            );
        }

        self.r_frame
            .connect(&format!("tcp://{}:{}", url, HOST_FRAME_STREAM_PORT))
            .expect(&format!(
                "[C] [{}] failed connecting to frame stream at {}",
                self.user_id,
                format!("tcp://{}:{}", url, HOST_FRAME_STREAM_PORT)
            ));

        self.r_frame
            .set_subscribe(b"frame")
            .expect("failed subscribing");
    }

    pub fn primary_read_envelope(&mut self) -> String {
        self.rw_primary
            .recv_string(0)
            .expect(&format!("[C] [{}] failed reading envelope 1", self.user_id))
            .expect(&format!("[C] [{}] failed reading envelope 2", self.user_id))
    }

    pub fn send<T>(&self, data: T, flags: i32) -> Result<(), zmq::Error>
    where
        T: zmq::Sendable,
    {
        self.input.send(data, flags)
    }

    pub fn send_input(&self, data: &[u8]) {
        self.send("input", zmq::SNDMORE)
            .expect("failed sending input envelope");
        self.send(data, 0).expect("failed sending input");
    }
}

pub fn host() {
    let context = zmq::Context::new();
    let publisher = context.socket(zmq::PUB).unwrap();
    publisher
        .bind("tcp://*:5563")
        .expect("failed binding publisher");

    loop {
        publisher
            .send("A", zmq::SNDMORE)
            .expect("failed sending frame envelope");
        publisher
            .send("We don't want to see this", 0)
            .expect("failed sending first message");
        publisher
            .send("B", zmq::SNDMORE)
            .expect("failed sending second envelope");
        publisher
            .send("We would like to see this", 0)
            .expect("failed sending second message");
        thread::sleep(Duration::from_millis(1));
    }
}

pub fn connect() {
    let context = zmq::Context::new();
    let subscriber = context.socket(zmq::SUB).unwrap();
    subscriber
        .connect("tcp://localhost:5563")
        .expect("failed connecting subscriber");
    subscriber
        .set_subscribe(b"frame")
        .expect("failed subscribing");

    loop {
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

        println!(
            "len: {} | {}x{} | image.start: {} | image.end: {}",
            message.len(),
            width,
            height,
            message[0],
            message[message.len() - 1]
        );

        write_to_png(width, height, &message);
    }
}

fn write_to_png(width: u32, height: u32, data: &[u8]) {
    // For reading and opening files
    use std::fs::File;
    use std::io::BufWriter;
    use std::path::Path;

    let path = Path::new(r"ss.png");
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width, height); // Width is 2 pixels and height is 1.
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_trns(vec![0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8]);
    encoder.set_source_gamma(png::ScaledFloat::from_scaled(45455)); // 1.0 / 2.2, scaled by 100000
    encoder.set_source_gamma(png::ScaledFloat::new(1.0 / 2.2)); // 1.0 / 2.2, unscaled, but rounded
    let source_chromaticities = png::SourceChromaticities::new(
        // Using unscaled instantiation here
        (0.31270, 0.32900),
        (0.64000, 0.33000),
        (0.30000, 0.60000),
        (0.15000, 0.06000),
    );
    encoder.set_source_chromaticities(source_chromaticities);
    let mut writer = encoder.write_header().unwrap();

    writer.write_image_data(data).unwrap(); // Save
}
