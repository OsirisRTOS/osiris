use crate::hal::{self, Machinelike};

use crate::{sched, sync};

static TICKS: sync::atomic::AtomicU64 = sync::atomic::AtomicU64::new(0);

extern "C" fn update_time() {
    let interval: u64 = 100_000; // ~100 seconds in ticks
    kprintln!(
        "Time thread started with tick interval {} at {}",
        interval,
        walltime()
    );
    loop {
        let tick = tick();
        sched::with(|sched| {
            let _ = sched.sleep_until(tick + interval, tick);
        });
        kprintln!("time is now {}", walltime());
    }
}

pub fn init() {
    let attrs = sched::thread::Attributes {
        entry: update_time,
        fin: None,
        attrs: None,
    };

    sched::with(|sched| {
        if let Ok(uid) = sched.create_thread(Some(sched::task::KERNEL_TASK), &attrs) {
            if sched.enqueue(tick(), uid).is_err() {
                panic!("failed to enqueue time thread.");
            }
        } else {
            panic!("failed to create time task.");
        }
    })
}

pub fn rtc_backup_register(index: u8) -> u32 {
    assert!(index < 32, "RTC backup register index out of bounds");
    assert!(index != 0, "RTC uses this register for restart continuity");
    hal::Machine::rtc_backup_register(index)
}

pub fn set_rtc_backup_register(index: u8, value: u32) {
    assert!(index < 32, "RTC backup register index out of bounds");
    assert!(index != 0, "RTC uses this register for restart continuity");
    hal::Machine::set_rtc_backup_register(index, value)
}

pub fn walltime() -> u64 {
    let raw = hal::Machine::rtc_raw();
    if raw == -1i64 as u64 {
        kprintln!("failed to read RTC time");
        return 0;
    }
    if raw == -2i64 as u64 {
        kprintln!("failed to read RTC date");
        return 0;
    }
    rtc_raw_to_unix(raw)
}

pub fn set_walltime(time: u64) {
    let raw = unix_to_rtc_raw(time);
    match hal::Machine::set_rtc_raw(raw) {
        0 => (),
        -1 => kprintln!("failed to set RTC time"),
        -2 => kprintln!("failed to set RTC date"),
        _ => kprintln!("unknown error setting RTC time"),
    }
}

const fn bcd_to_bin(value: u8) -> u8 {
    ((value >> 4) * 10) + (value & 0x0f)
}

const fn bin_to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
}

const fn is_leap_year(year: u32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

const fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn days_since_unix_epoch(year: u32, month: u32, day: u32) -> u64 {
    let mut days = 0u64;
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    for m in 1..month {
        days += days_in_month(year, m) as u64;
    }
    days + u64::from(day.saturating_sub(1))
}

fn rtc_raw_to_unix(raw: u64) -> u64 {
    let hours = bcd_to_bin((raw & 0xff) as u8) as u64;
    let minutes = bcd_to_bin(((raw >> 8) & 0xff) as u8) as u64;
    let seconds = bcd_to_bin(((raw >> 16) & 0xff) as u8) as u64;
    let year = 2000 + u32::from(bcd_to_bin(((raw >> 56) & 0xff) as u8));
    let month = u32::from(bcd_to_bin(((raw >> 40) & 0xff) as u8));
    let day = u32::from(bcd_to_bin(((raw >> 48) & 0xff) as u8));

    days_since_unix_epoch(year, month, day) * 86_400 + hours * 3_600 + minutes * 60 + seconds
}

fn unix_to_rtc_raw(unix: u64) -> u64 {
    let epoch_2000 = 946_684_800u64;
    let mut seconds = unix.saturating_sub(epoch_2000);
    let mut days = seconds / 86_400;
    seconds %= 86_400;

    let mut year = 2000u32;
    while days >= if is_leap_year(year) { 366 } else { 365 } {
        days -= if is_leap_year(year) { 366 } else { 365 };
        year += 1;
    }

    let mut month = 1u32;
    while days >= u64::from(days_in_month(year, month)) {
        days -= u64::from(days_in_month(year, month));
        month += 1;
    }

    let day = (days + 1) as u32;
    let weekday = (((days_since_unix_epoch(year, month, day) + 4) % 7) + 1) as u8;

    let hours = (seconds / 3_600) as u8;
    seconds %= 3_600;
    let minutes = (seconds / 60) as u8;
    let seconds = (seconds % 60) as u8;

    (u64::from(bin_to_bcd(hours)))
        | (u64::from(bin_to_bcd(minutes)) << 8)
        | (u64::from(bin_to_bcd(seconds)) << 16)
        | (u64::from(weekday) << 32)
        | (u64::from(bin_to_bcd(month as u8)) << 40)
        | (u64::from(bin_to_bcd(day as u8)) << 48)
        | (u64::from(bin_to_bcd((year - 2000) as u8)) << 56)
}

pub fn tick() -> u64 {
    TICKS.load(sync::atomic::Ordering::Acquire)
}

pub fn mono_now() -> u64 {
    // TODO: This will break on SMP systems without native u64 atomic store.
    sync::atomic::irq_free(|| hal::Machine::monotonic_now())
}

pub fn mono_freq() -> u64 {
    hal::Machine::monotonic_freq()
}

/// cbindgen:ignore
/// cbindgen:no-export
#[unsafe(no_mangle)]
pub extern "C" fn systick_hndlr() {
    let tick = TICKS.fetch_add(1, sync::atomic::Ordering::Release) + 1;

    if sched::needs_reschedule(tick) {
        sched::reschedule();
    }
}
