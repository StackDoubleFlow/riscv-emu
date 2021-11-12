mod core;

use crate::core::Core;

fn main() {
    let mut core = Core::new();
    core.load_image(std::fs::read("test/image.bin").unwrap());
    core.run();
    // for _ in 0..100 {
    //     core.step();
    // }
}
