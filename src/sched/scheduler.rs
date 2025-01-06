use hal::common::sched::CtxPtr;

extern "C" fn schedule(ctx: CtxPtr) -> ! {
    unreachable!()
}

fn update_thread(ctx: CtxPtr) {}
