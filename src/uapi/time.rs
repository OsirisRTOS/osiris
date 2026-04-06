use crate::time;

pub fn mono_now() -> u64 {
    time::mono_now()
}

pub fn tick() -> u64 {
    time::tick()
}
