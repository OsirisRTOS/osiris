use crate::time;

pub fn mono_now() -> u64 {
    time::mono_now()
}

pub fn mono_freq() -> u64 {
    time::mono_freq()
}

pub fn tick() -> u64 {
    time::tick()
}
