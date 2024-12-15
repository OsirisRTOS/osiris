enum TaskState {
    Running,
    Ready,
    Blocked,
    Finished,
}

pub struct Task {
    pub id: u32,
    pub threads: [u32; 16],
}
