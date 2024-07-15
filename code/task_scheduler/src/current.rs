use alloc::sync::Arc;

#[percpu::def_percpu]
static CURRENT_TASK_PTR: usize = 0;


pub fn get_current<T>() -> Option<Arc<T>> {
    
}

pub fn replace_current<T>(new_task: T) -> T {

}