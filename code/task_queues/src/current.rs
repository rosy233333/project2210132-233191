use core::mem;

use alloc::sync::Arc;

pub struct CurrentTask<T>(Arc<T>);

impl<T> CurrentTask<T> {
    pub fn new(task: Arc<T>) -> Self {
        Self {
            0: task
        }
    }

    /// 获取当前任务的Arc指针（Current中会保留当前任务）
    pub fn get_current_ptr(&self) -> Arc<T> {
        self.0.clone()
    }

    /// 替换任务，上一个任务作为函数返回值传出。
    pub fn replace_current(&mut self, new_task: Arc<T>) -> Arc<T> {
        mem::replace(&mut self.0, new_task)
    }
}

// static CURRENT_TASK_PTR: usize = 0;

// /// 获取当前任务的Arc指针（Current中会保留当前任务）
// pub fn get_current_ptr<T>() -> Option<Arc<T>> {
//     let current_ptr = CURRENT_TASK_PTR.read_current();
//     if current_ptr != 0 {
//         let current_arc_ptr: Arc<T> = unsafe {
//             Arc::from_raw(current_ptr as *const () as *const T)
//         };
//         let return_arc_ptr = current_arc_ptr.clone();
//         Arc::into_raw(current_arc_ptr); // 防止Current拥有的Arc被释放
//         Some(return_arc_ptr)
//     }
//     else {
//         None
//     }
// }

// /// 替换任务，上一个任务作为函数返回值传出。
// pub fn replace_current<T>(new_task: Arc<T>) -> Option<Arc<T>> {
//     let former_ptr = CURRENT_TASK_PTR.read_current();
//     CURRENT_TASK_PTR.write_current(Arc::into_raw(new_task) as *const () as usize);
//     if former_ptr != 0 {
//         let former_arc_ptr: Arc<T> = unsafe {
//             Arc::from_raw(former_ptr as *const () as *const T)
//         };
//         Some(former_arc_ptr)
//     }
//     else {
//         None
//     }
// }