use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering::Relaxed;

fn main() {
    let ptr = &mut 5;
    let atomic_ptr = AtomicPtr::new(ptr);
    let i1 = unsafe { *atomic_ptr.load(Relaxed) };
    let i2 = unsafe { *atomic_ptr.load(Relaxed) };

    println!("{:?}", i1);
    println!("{:?}", i2);
}