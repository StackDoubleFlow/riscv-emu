mod core;
mod mem;

use crate::core::Core;

fn main() {
    let mut core = Core::new();
    core.load_image(std::fs::read("test/sbi/image.bin").unwrap());
    core.run();
    // for _ in 0..100 {
    //     core.step();
    // }
}
