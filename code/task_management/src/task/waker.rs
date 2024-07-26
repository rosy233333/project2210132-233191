//! This mod specific the waker related with coroutine
//!

use super::Task;
use alloc::sync::Arc;

use core::task::{RawWaker, RawWakerVTable, Waker};

const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake, drop);

unsafe fn clone(p: *const ()) -> RawWaker {
    RawWaker::new(p, &VTABLE)
}

/// nop
// 疑问：这里的wake函数是否会释放掉持有的任务的Arc？如果是的话，它是否不能直接作为wake_by_ref函数？
unsafe fn wake(p: *const ()) { 
    Arc::from_raw(p as *const Task).wakeup();
}

unsafe fn drop(p: *const ()) {
    // nop
    Arc::from_raw(p as *const Task);
}

/// 
pub(crate) fn waker_from_task(task_ref: Arc<Task>) -> Waker {
    unsafe {
        Waker::from_raw(RawWaker::new(Arc::into_raw(task_ref) as _, &VTABLE))
    }
}