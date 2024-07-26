// extern crate alloc;
use core::{alloc::Layout, ptr::NonNull};

const TASK_STACK_SIZE: usize = 0x40000;

pub(crate) struct TaskStack {
    ptr: NonNull<u8>,
    layout: Layout,
}

impl TaskStack {
    pub fn alloc() -> Self {
        let layout = Layout::from_size_align(TASK_STACK_SIZE, 16).unwrap();
        Self {
            ptr: NonNull::new(unsafe { alloc::alloc::alloc(layout) }).unwrap(),
            layout,
        }
    }

    pub const fn top(&self) -> usize {
        unsafe { core::mem::transmute(self.ptr.as_ptr().add(self.layout.size())) }
    }

    // #[cfg(feature = "monolithic")]
    // /// 获取内核栈第一个压入的trap上下文，防止出现内核trap嵌套
    // pub fn get_first_trap_frame(&self) -> *mut TrapFrame {
    //     (self.top().as_usize() - core::mem::size_of::<TrapFrame>()) as *mut TrapFrame
    // }
}

impl Drop for TaskStack {
    fn drop(&mut self) {
        unsafe { alloc::alloc::dealloc(self.ptr.as_ptr(), self.layout) }
    }
}