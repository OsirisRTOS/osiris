use crate::{
    error::Result,
    sync::{self, spinlock::RwSpinLocked},
    types::array::Vec,
};

/// The IRQ handler type. The first argument is a pointer to the context, the second argument is the IRQ vector, and the third argument is userdata.
pub type IrqHandler = fn(*mut u8, usize, Option<usize>);

#[derive(Clone, Copy)]
struct Handler {
    func: IrqHandler,
    userdata: Option<usize>,
}

// TODO: Amount of lines should be configured by the DeviceTree.
static HANDLERS: [RwSpinLocked<Vec<Handler, 1>>; 240] =
    [const { RwSpinLocked::new(Vec::new()) }; 240];

/// Register an IRQ handler for the given vector.
///
/// `vector` - The IRQ vector to register the handler for.
/// `handler` - The IRQ handler to register.
/// `userdata` - Optional userdata to pass to the handler when the IRQ is triggered.
///
/// # Safety
/// - The caller must ensure that the handler is safe to call in an IRQ context and that the userdata is valid for the lifetime of the handler.
/// - This functions must not be called from an IRQ context.
pub unsafe fn register_irq(
    vector: usize,
    handler: IrqHandler,
    userdata: Option<usize>,
) -> Result<()> {
    if vector >= HANDLERS.len() {
        Err(kerr!(InvalidArgument, "Invalid IRQ vector."))?;
    }

    let handler = Handler {
        func: handler,
        userdata,
    };

    // If an irq happens while HANDLERS is locked this will deadlock.
    // Thats why we need to modify it in an irq free section.
    sync::atomic::irq_free(|| HANDLERS[vector].write_lock().push(handler))
}

/// Unregister all IRQ handlers for the given vector.
///
/// `vector` - The IRQ vector to unregister the handlers for.
///
/// # Safety
/// - This function must not be called from an IRQ context.
pub unsafe fn unregister_irq(vector: usize) -> Result<()> {
    if vector >= HANDLERS.len() {
        Err(kerr!(InvalidArgument, "Invalid IRQ vector."))?;
    }

    sync::atomic::irq_free(|| {
        HANDLERS[vector].write_lock().clear();
    });

    Ok(())
}

#[unsafe(no_mangle)]
extern "C" fn kernel_irq_handler(ctx: *mut u8, vector: usize) {
    if vector >= HANDLERS.len() {
        warn!("Invalid IRQ vector {}", vector);
        return;
    }

    // It is forbidden to hold a HANDLERS write_lock in an irq context.
    let handler = HANDLERS[vector].read_lock();

    if handler.is_empty() {
        warn!("Unhandled IRQ {}", vector);
        return;
    }

    for i in 0..handler.len() {
        let handler = handler[i];
        (handler.func)(ctx, vector, handler.userdata)
    }
}
