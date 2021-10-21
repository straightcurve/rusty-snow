use std::thread;
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt};

pub struct Host {
    context: zmq::Context,
    out: zmq::Socket,
}

impl Host {
    pub fn new() -> Self {
        let context = zmq::Context::new();
        let out = context.socket(zmq::PUB).unwrap();
        out.bind("tcp://*:5563")
            .expect("Failed binding out socket for host");

        Self { context, out }
    }

    pub fn send<T>(&self, data: T, flags: i32) -> Result<(), zmq::Error>
    where
        T: zmq::Sendable,
    {
        self.out.send(data, flags)
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
