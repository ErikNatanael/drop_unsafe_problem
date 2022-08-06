use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

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
    drop_count: Arc<AtomicUsize>,
    drop_flag: Arc<AtomicBool>,
}
impl World {
    fn create_audio_task(&mut self) -> AudioThreadTask {
        self.drop_count.fetch_add(1, Ordering::Relaxed);
        let nodes = self
            .owned
            .iter_mut()
            .map(|n| (&mut **n) as *mut Node) //(&mut (*n)) as *mut Node)
            .collect();
        AudioThreadTask {
            nodes,
            drop_count: self.drop_count.clone(),
            drop_flag: self.drop_flag.clone(),
        }
    }
}

impl Drop for World {
    fn drop(&mut self) {
        // Setting the flag here doesn't work, the change isn't visible to the AudioThreadTask
        self.drop_flag.store(true, Ordering::Relaxed);
        println!(
            "Dropping World, flag: {}",
            self.drop_flag.load(Ordering::Relaxed)
        );
        let mut last_drop_count = self.drop_count.load(Ordering::SeqCst);
        while last_drop_count > 0 {
            let new_drop_count = self.drop_count.load(Ordering::Relaxed);
            if last_drop_count > new_drop_count {
                println!("drop count decreased: {new_drop_count}");
            }
            last_drop_count = new_drop_count;
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

struct AudioThreadTask {
    nodes: Vec<*mut Node>,
    drop_count: Arc<AtomicUsize>,
    drop_flag: Arc<AtomicBool>,
}

unsafe impl Send for AudioThreadTask {}

impl AudioThreadTask {
    fn process(&mut self, output: &mut f32) -> bool {
        for node in &mut self.nodes {
            let node = unsafe { &mut **node };
            *output += node.gen.process();
        }
        if self.drop_flag.load(Ordering::Relaxed) {
            self.drop_count.fetch_sub(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
}

fn create_world() -> World {
    let mut world = World {
        owned: vec![],
        drop_count: Arc::new(AtomicUsize::new(0)),
        drop_flag: Arc::new(AtomicBool::new(false)),
    };
    for _ in 0..10 {
        let osc = Osc { data: 0.0 };
        let node = Node { gen: Box::new(osc) };
        world.owned.push(Box::new(node));
    }
    world
}

fn main() {
    let mut world = create_world();
    let mut audio_thread_task = world.create_audio_task();

    // This solution works when audio_thread_task is run on a different thread.
    std::thread::spawn(move || loop {
        // The "audio thread". In reality, output would be piped to the sound card here
        let mut value = 0.0;
        let release = audio_thread_task.process(&mut value);
        if release {
            return;
        }
        println!("{value}");
        std::thread::sleep(std::time::Duration::from_millis(100));
    });

    println!("Audio thread started");

    std::thread::sleep(std::time::Duration::from_millis(1000));
    // This sets the atomic so that the AudioThreadTask sees it
    // world.drop_flag.store(true, Ordering::Relaxed);
    // std::thread::sleep(std::time::Duration::from_millis(1000));

    drop(world); // World gets dropped while the audio thread may be accessing the owned data
    println!("World has been dropped");
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // But if both are owned by the same thread, there is a risk of blocking forever

    println!("Same thread");
    let mut world = create_world();
    let mut audio_thread_task = world.create_audio_task();

    for _ in 0..10 {
        let mut value = 0.0;
        let release = audio_thread_task.process(&mut value);
        println!("{value}");
        if release {
            break;
        }
    }

    drop(world);
    println!("World has been dropped");
    std::thread::sleep(std::time::Duration::from_millis(1000));
}
