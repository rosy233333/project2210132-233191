use alloc::sync::Arc;

#[percpu::def_percpu]
static CURRENT_TASK_PTR: usize = 0;

/// 在使用current相关内容前，需要先在每个CPU核心调用下面两个函数之一。
/// 主CPU调用第一个函数，其它CPU调用第二个函数。
pub fn main_cpu_init(cpu_id: usize, cpu_num: usize) {
    percpu::init(cpu_num);
    percpu::set_local_thread_pointer(cpu_id);
}

pub fn secondary_cpu_init(cpu_id: usize) {
    percpu::set_local_thread_pointer(cpu_id);
}

/// 获取当前任务的Arc指针（Current中会保留当前任务）
pub fn get_current_ptr<T>() -> Option<Arc<T>> {
    let current_ptr = CURRENT_TASK_PTR.read_current();
    if current_ptr != 0 {
        let current_arc_ptr: Arc<T> = unsafe {
            Arc::from_raw(current_ptr as *const () as *const T)
        };
        let return_arc_ptr = current_arc_ptr.clone();
        Arc::into_raw(current_arc_ptr); // 防止Current拥有的Arc被释放
        Some(return_arc_ptr)
    }
    else {
        None
    }
}

/// 替换任务，上一个任务作为函数返回值传出。
pub fn replace_current<T>(new_task: Arc<T>) -> Option<Arc<T>> {
    let former_ptr = CURRENT_TASK_PTR.read_current();
    CURRENT_TASK_PTR.write_current(Arc::into_raw(new_task) as *const () as usize);
    if former_ptr != 0 {
        let former_arc_ptr: Arc<T> = unsafe {
            Arc::from_raw(former_ptr as *const () as *const T)
        };
        Some(former_arc_ptr)
    }
    else {
        None
    }
}