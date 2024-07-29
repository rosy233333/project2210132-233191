use alloc::sync::Arc;
use super::TaskStack;
use alloc::vec::Vec;

/// A simple stack pool
pub(crate) struct StackPool {
    curr_stack: Option<Arc<TaskStack>>,
    free_stacks: Vec<Arc<TaskStack>>,
}

impl StackPool {
    /// Creates a new empty stack pool.
    pub(crate) const fn new() -> Self {
        Self {
            curr_stack: None,
            free_stacks: Vec::new(),
        }
    }

    /// Fetch a free stack from the pool.
    pub(crate) fn fetch(&mut self) -> Arc<TaskStack> {
        self.free_stacks.pop().unwrap_or_else(|| Arc::new(TaskStack::alloc()))
    }

    /// Set current stack.
    pub(crate) fn swap_curr_stack(&mut self, stack: Option<Arc<TaskStack>>) -> Option<Arc<TaskStack>> {
        // if let Some(old_stack) = self.curr_stack.take() {
        //     self.free_stacks.push(old_stack);
        // }
        // if let Some(stack) = stack {
        //     self.curr_stack.replace(stack);
        // }
        let old_stack = self.curr_stack.take();
        self.curr_stack = stack;
        old_stack
    }

    /// Recycle an empty stack.
    /// SAFETY: the recycled stack must be empty and no longer used by a thread.
    pub(crate) unsafe fn recycle_stack(&mut self, empty_stack: Arc<TaskStack>) {
        self.free_stacks.push(empty_stack);
    }
}