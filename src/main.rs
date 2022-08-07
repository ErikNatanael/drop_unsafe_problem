use std::{cell::UnsafeCell, sync::Arc};

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
    owned: Arc<UnsafeCell<Vec<Box<Node>>>>,
}
impl World {
    fn create_audio_task(&mut self) -> AudioThreadTask {
        let nodes = unsafe { &mut *self.owned.get() };
        let nodes = nodes
            .iter_mut()
            .map(|n| (&mut **n) as *mut Node) //(&mut (*n)) as *mut Node)
            .collect();
        AudioThreadTask {
            nodes,
            _arc_nodes: self.owned.clone(),
        }
    }
    fn push(&mut self, node: Node) {
        unsafe { &mut *self.owned.get() }.push(Box::new(node))
    }
}

struct AudioThreadTask {
    nodes: Vec<*mut Node>,
    // This exists only so that the nodes won't get dropped.
    _arc_nodes: Arc<UnsafeCell<Vec<Box<Node>>>>,
}

unsafe impl Send for AudioThreadTask {}

impl AudioThreadTask {
    fn process(&mut self, output: &mut f32) {
        for node in &self.nodes {
            let node = unsafe { &mut **node };
            *output += node.gen.process();
        }
    }
}

fn main() {
    let osc = Osc { data: 0.0 };
    let node = Node { gen: Box::new(osc) };
    let mut world = World {
        owned: Arc::new(UnsafeCell::new(vec![])),
    };
    world.push(node);
    let mut audio_thread_task = world.create_audio_task();
    std::thread::spawn(move || loop {
        // The "audio thread". In reality, output would be piped to the sound card here
        let mut value = 0.0;
        audio_thread_task.process(&mut value);
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
