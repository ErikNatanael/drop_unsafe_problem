trait Gen {
    fn process(&mut self) -> f32;
}

struct Osc {
    data: f32,
}

impl Gen for Osc {
    fn process(&mut self) -> f32 {
        self.data += 0.1;
        self.data
    }
}

struct Node {
    gen: Box<dyn Gen>,
}

struct World {
    owned: Vec<Box<Node>>,
}
impl World {
    fn create_audio_task(&mut self) -> AudioThreadTask {
        AudioThreadTask {
            node: &mut (*self.owned[0]) as *mut Node,
        }
    }
}

struct AudioThreadTask {
    node: *mut Node,
}

unsafe impl Send for AudioThreadTask {}

impl AudioThreadTask {
    fn process(&mut self) -> f32 {
        let node = unsafe { &mut *self.node };
        node.gen.process()
    }
}

fn main() {
    let osc = Osc { data: 0.0 };
    let node = Node { gen: Box::new(osc) };
    let mut world = World { owned: vec![] };
    world.owned.push(Box::new(node));
    let mut audio_thread_task = world.create_audio_task();
    std::thread::spawn(move || loop {
        // The "audio thread". In reality, output would be piped to the sound card here
        let value = audio_thread_task.process();
        println!("{value}");
        std::thread::sleep(std::time::Duration::from_millis(100));
    });

    println!("Audio thread started");

    std::thread::sleep(std::time::Duration::from_millis(1000));
    // do stuff a lot of stuff

    drop(world); // World gets dropped while the audio thread may be accessing the owned data
    println!("World has been dropped");
    std::thread::sleep(std::time::Duration::from_millis(3000));
}
