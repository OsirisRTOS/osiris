use super::task::Task;

pub fn prepare(task: &mut Task) {
    if task.id.is_kernel() {
        // Change task priv. level in HAL.
    }
}
