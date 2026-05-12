#![cfg_attr(not(test), no_std)]

use core::fmt;
use core::fmt::Display;
pub mod mem;
pub mod stack;

/// POSIX / Linux errno values 1–133.
///
/// The sequence is contiguous except for two slots that are pure C-macro
/// aliases in Linux and therefore carry no unique meaning:
///   - 41  (`EWOULDBLOCK` = `EAGAIN` = 11)
///   - 58  (`EDEADLOCK`   = `EDEADLK` = 35)
/// Those are represented as `Reserved41` / `Reserved58`.
///
/// `from_errno` / `From<i32>` convert raw OS errno values; unrecognised
/// values map to `Unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PosixError {
    // --- 1–40: universally-portable base set ---
    /// Operation not permitted
    EPERM = 1,
    /// No such file or directory
    ENOENT = 2,
    /// No such process
    ESRCH = 3,
    /// Interrupted system call
    EINTR = 4,
    /// Input/output error
    EIO = 5,
    /// No such device or address
    ENXIO = 6,
    /// Argument list too long
    E2BIG = 7,
    /// Exec format error
    ENOEXEC = 8,
    /// Bad file descriptor
    EBADF = 9,
    /// No child processes
    ECHILD = 10,
    /// Resource temporarily unavailable (EWOULDBLOCK is a C alias for this)
    EAGAIN = 11,
    /// Cannot allocate memory
    ENOMEM = 12,
    /// Permission denied
    EACCES = 13,
    /// Bad address
    EFAULT = 14,
    /// Block device required
    ENOTBLK = 15,
    /// Device or resource busy
    EBUSY = 16,
    /// File exists
    EEXIST = 17,
    /// Invalid cross-device link
    EXDEV = 18,
    /// No such device
    ENODEV = 19,
    /// Not a directory
    ENOTDIR = 20,
    /// Is a directory
    EISDIR = 21,
    /// Invalid argument
    EINVAL = 22,
    /// Too many open files in system
    ENFILE = 23,
    /// Too many open files
    EMFILE = 24,
    /// Inappropriate ioctl for device
    ENOTTY = 25,
    /// Text file busy
    ETXTBSY = 26,
    /// File too large
    EFBIG = 27,
    /// No space left on device
    ENOSPC = 28,
    /// Illegal seek
    ESPIPE = 29,
    /// Read-only file system
    EROFS = 30,
    /// Too many links
    EMLINK = 31,
    /// Broken pipe
    EPIPE = 32,
    /// Numerical argument out of domain
    EDOM = 33,
    /// Numerical result out of range
    ERANGE = 34,
    /// Resource deadlock avoided (EDEADLOCK is a C alias for this)
    EDEADLK = 35,
    /// File name too long
    ENAMETOOLONG = 36,
    /// No locks available
    ENOLCK = 37,
    /// Function not implemented
    ENOSYS = 38,
    /// Directory not empty
    ENOTEMPTY = 39,
    /// Too many levels of symbolic links
    ELOOP = 40,
    /// Reserved — slot occupied by the EWOULDBLOCK alias (= EAGAIN = 11) in C
    Reserved41 = 41,
    /// No message of desired type
    ENOMSG = 42,
    /// Identifier removed
    EIDRM = 43,
    // --- 44–59: SysV / STREAMS legacy codes ---
    /// Channel number out of range
    ECHRNG = 44,
    /// Level 2 not synchronised
    EL2NSYNC = 45,
    /// Level 3 halted
    EL3HLT = 46,
    /// Level 3 reset
    EL3RST = 47,
    /// Link number out of range
    ELNRNG = 48,
    /// Protocol driver not attached
    EUNATCH = 49,
    /// No CSI structure available
    ENOCSI = 50,
    /// Level 2 halted
    EL2HLT = 51,
    /// Invalid exchange
    EBADE = 52,
    /// Invalid request descriptor
    EBADR = 53,
    /// Exchange full
    EXFULL = 54,
    /// No anode
    ENOANO = 55,
    /// Invalid request code
    EBADRQC = 56,
    /// Invalid slot
    EBADSLT = 57,
    /// Reserved — slot occupied by the EDEADLOCK alias (= EDEADLK = 35) in C
    Reserved58 = 58,
    /// Bad font file format
    EBFONT = 59,
    // --- 60–87: STREAMS + misc ---
    /// Device not a stream
    ENOSTR = 60,
    /// No data available
    ENODATA = 61,
    /// Timer expired
    ETIME = 62,
    /// Out of streams resources
    ENOSR = 63,
    /// Machine is not on the network
    ENONET = 64,
    /// Package not installed
    ENOPKG = 65,
    /// Object is remote
    EREMOTE = 66,
    /// Link has been severed
    ENOLINK = 67,
    /// Advertise error
    EADV = 68,
    /// Srmount error
    ESRMNT = 69,
    /// Communication error on send
    ECOMM = 70,
    /// Protocol error
    EPROTO = 71,
    /// Multihop attempted
    EMULTIHOP = 72,
    /// RFS specific error
    EDOTDOT = 73,
    /// Bad message
    EBADMSG = 74,
    /// Value too large for defined data type
    EOVERFLOW = 75,
    /// Name not unique on network
    ENOTUNIQ = 76,
    /// File descriptor in bad state
    EBADFD = 77,
    /// Remote address changed
    EREMCHG = 78,
    /// Cannot access a needed shared library
    ELIBACC = 79,
    /// Accessing a corrupted shared library
    ELIBBAD = 80,
    /// .lib section in a.out corrupted
    ELIBSCN = 81,
    /// Attempting to link in too many shared libraries
    ELIBMAX = 82,
    /// Cannot exec a shared library directly
    ELIBEXEC = 83,
    /// Invalid or incomplete multibyte or wide character
    EILSEQ = 84,
    /// Interrupted syscall should be restarted
    ERESTART = 85,
    /// Streams pipe error
    ESTRPIPE = 86,
    /// Too many users
    EUSERS = 87,
    // --- 88–122: sockets / networking ---
    /// Socket operation on non-socket
    ENOTSOCK = 88,
    /// Destination address required
    EDESTADDRREQ = 89,
    /// Message too long
    EMSGSIZE = 90,
    /// Protocol wrong type for socket
    EPROTOTYPE = 91,
    /// Protocol not available
    ENOPROTOOPT = 92,
    /// Protocol not supported
    EPROTONOSUPPORT = 93,
    /// Socket type not supported
    ESOCKTNOSUPPORT = 94,
    /// Operation not supported
    EOPNOTSUPP = 95,
    /// Protocol family not supported
    EPFNOSUPPORT = 96,
    /// Address family not supported by protocol
    EAFNOSUPPORT = 97,
    /// Address already in use
    EADDRINUSE = 98,
    /// Cannot assign requested address
    EADDRNOTAVAIL = 99,
    /// Network is down
    ENETDOWN = 100,
    /// Network is unreachable
    ENETUNREACH = 101,
    /// Network dropped connection on reset
    ENETRESET = 102,
    /// Software caused connection abort
    ECONNABORTED = 103,
    /// Connection reset by peer
    ECONNRESET = 104,
    /// No buffer space available
    ENOBUFS = 105,
    /// Transport endpoint is already connected
    EISCONN = 106,
    /// Transport endpoint is not connected
    ENOTCONN = 107,
    /// Cannot send after transport endpoint shutdown
    ESHUTDOWN = 108,
    /// Too many references: cannot splice
    ETOOMANYREFS = 109,
    /// Connection timed out
    ETIMEDOUT = 110,
    /// Connection refused
    ECONNREFUSED = 111,
    /// Host is down
    EHOSTDOWN = 112,
    /// No route to host
    EHOSTUNREACH = 113,
    /// Operation already in progress
    EALREADY = 114,
    /// Operation now in progress
    EINPROGRESS = 115,
    /// Stale file handle
    ESTALE = 116,
    /// Structure needs cleaning
    EUCLEAN = 117,
    /// Not a XENIX named type file
    ENOTNAM = 118,
    /// No XENIX semaphores available
    ENAVAIL = 119,
    /// Is a named type file
    EISNAM = 120,
    /// Remote I/O error
    EREMOTEIO = 121,
    /// Disk quota exceeded
    EDQUOT = 122,
    // --- 123–133: modern additions ---
    /// No medium found
    ENOMEDIUM = 123,
    /// Wrong medium type
    EMEDIUMTYPE = 124,
    /// Operation cancelled
    ECANCELED = 125,
    /// Required key not available
    ENOKEY = 126,
    /// Key has expired
    EKEYEXPIRED = 127,
    /// Key has been revoked
    EKEYREVOKED = 128,
    /// Key was rejected by service
    EKEYREJECTED = 129,
    /// Owner died (robust mutexes)
    EOWNERDEAD = 130,
    /// State not recoverable (robust mutexes)
    ENOTRECOVERABLE = 131,
    /// Operation not possible due to RF-kill
    ERFKILL = 132,
    /// Memory page has hardware error
    EHWPOISON = 133,
    /// Unknown or unrepresentable error code
    Unknown = -1,
}

impl PosixError {
    /// Construct from a raw `errno` integer. Returns `Unknown` for unrecognised values.
    pub fn from_errno(errno: i32) -> Self {
        if matches!(errno, 1..=133) {
            // SAFETY: every value in 1..=133 is a valid discriminant of this
            // `#[repr(i32)]` enum — the variants are contiguous and complete.
            unsafe { core::mem::transmute(errno) }
        } else {
            Self::Unknown
        }
    }

    /// The canonical C errno name (e.g. `"ENOENT"`).
    pub fn name(self) -> &'static str {
        match self {
            Self::EPERM => "EPERM",
            Self::ENOENT => "ENOENT",
            Self::ESRCH => "ESRCH",
            Self::EINTR => "EINTR",
            Self::EIO => "EIO",
            Self::ENXIO => "ENXIO",
            Self::E2BIG => "E2BIG",
            Self::ENOEXEC => "ENOEXEC",
            Self::EBADF => "EBADF",
            Self::ECHILD => "ECHILD",
            Self::EAGAIN => "EAGAIN",
            Self::ENOMEM => "ENOMEM",
            Self::EACCES => "EACCES",
            Self::EFAULT => "EFAULT",
            Self::ENOTBLK => "ENOTBLK",
            Self::EBUSY => "EBUSY",
            Self::EEXIST => "EEXIST",
            Self::EXDEV => "EXDEV",
            Self::ENODEV => "ENODEV",
            Self::ENOTDIR => "ENOTDIR",
            Self::EISDIR => "EISDIR",
            Self::EINVAL => "EINVAL",
            Self::ENFILE => "ENFILE",
            Self::EMFILE => "EMFILE",
            Self::ENOTTY => "ENOTTY",
            Self::ETXTBSY => "ETXTBSY",
            Self::EFBIG => "EFBIG",
            Self::ENOSPC => "ENOSPC",
            Self::ESPIPE => "ESPIPE",
            Self::EROFS => "EROFS",
            Self::EMLINK => "EMLINK",
            Self::EPIPE => "EPIPE",
            Self::EDOM => "EDOM",
            Self::ERANGE => "ERANGE",
            Self::EDEADLK => "EDEADLK",
            Self::ENAMETOOLONG => "ENAMETOOLONG",
            Self::ENOLCK => "ENOLCK",
            Self::ENOSYS => "ENOSYS",
            Self::ENOTEMPTY => "ENOTEMPTY",
            Self::ELOOP => "ELOOP",
            Self::Reserved41 => "Reserved41",
            Self::ENOMSG => "ENOMSG",
            Self::EIDRM => "EIDRM",
            Self::ECHRNG => "ECHRNG",
            Self::EL2NSYNC => "EL2NSYNC",
            Self::EL3HLT => "EL3HLT",
            Self::EL3RST => "EL3RST",
            Self::ELNRNG => "ELNRNG",
            Self::EUNATCH => "EUNATCH",
            Self::ENOCSI => "ENOCSI",
            Self::EL2HLT => "EL2HLT",
            Self::EBADE => "EBADE",
            Self::EBADR => "EBADR",
            Self::EXFULL => "EXFULL",
            Self::ENOANO => "ENOANO",
            Self::EBADRQC => "EBADRQC",
            Self::EBADSLT => "EBADSLT",
            Self::Reserved58 => "Reserved58",
            Self::EBFONT => "EBFONT",
            Self::ENOSTR => "ENOSTR",
            Self::ENODATA => "ENODATA",
            Self::ETIME => "ETIME",
            Self::ENOSR => "ENOSR",
            Self::ENONET => "ENONET",
            Self::ENOPKG => "ENOPKG",
            Self::EREMOTE => "EREMOTE",
            Self::ENOLINK => "ENOLINK",
            Self::EADV => "EADV",
            Self::ESRMNT => "ESRMNT",
            Self::ECOMM => "ECOMM",
            Self::EPROTO => "EPROTO",
            Self::EMULTIHOP => "EMULTIHOP",
            Self::EDOTDOT => "EDOTDOT",
            Self::EBADMSG => "EBADMSG",
            Self::EOVERFLOW => "EOVERFLOW",
            Self::ENOTUNIQ => "ENOTUNIQ",
            Self::EBADFD => "EBADFD",
            Self::EREMCHG => "EREMCHG",
            Self::ELIBACC => "ELIBACC",
            Self::ELIBBAD => "ELIBBAD",
            Self::ELIBSCN => "ELIBSCN",
            Self::ELIBMAX => "ELIBMAX",
            Self::ELIBEXEC => "ELIBEXEC",
            Self::EILSEQ => "EILSEQ",
            Self::ERESTART => "ERESTART",
            Self::ESTRPIPE => "ESTRPIPE",
            Self::EUSERS => "EUSERS",
            Self::ENOTSOCK => "ENOTSOCK",
            Self::EDESTADDRREQ => "EDESTADDRREQ",
            Self::EMSGSIZE => "EMSGSIZE",
            Self::EPROTOTYPE => "EPROTOTYPE",
            Self::ENOPROTOOPT => "ENOPROTOOPT",
            Self::EPROTONOSUPPORT => "EPROTONOSUPPORT",
            Self::ESOCKTNOSUPPORT => "ESOCKTNOSUPPORT",
            Self::EOPNOTSUPP => "EOPNOTSUPP",
            Self::EPFNOSUPPORT => "EPFNOSUPPORT",
            Self::EAFNOSUPPORT => "EAFNOSUPPORT",
            Self::EADDRINUSE => "EADDRINUSE",
            Self::EADDRNOTAVAIL => "EADDRNOTAVAIL",
            Self::ENETDOWN => "ENETDOWN",
            Self::ENETUNREACH => "ENETUNREACH",
            Self::ENETRESET => "ENETRESET",
            Self::ECONNABORTED => "ECONNABORTED",
            Self::ECONNRESET => "ECONNRESET",
            Self::ENOBUFS => "ENOBUFS",
            Self::EISCONN => "EISCONN",
            Self::ENOTCONN => "ENOTCONN",
            Self::ESHUTDOWN => "ESHUTDOWN",
            Self::ETOOMANYREFS => "ETOOMANYREFS",
            Self::ETIMEDOUT => "ETIMEDOUT",
            Self::ECONNREFUSED => "ECONNREFUSED",
            Self::EHOSTDOWN => "EHOSTDOWN",
            Self::EHOSTUNREACH => "EHOSTUNREACH",
            Self::EALREADY => "EALREADY",
            Self::EINPROGRESS => "EINPROGRESS",
            Self::ESTALE => "ESTALE",
            Self::EUCLEAN => "EUCLEAN",
            Self::ENOTNAM => "ENOTNAM",
            Self::ENAVAIL => "ENAVAIL",
            Self::EISNAM => "EISNAM",
            Self::EREMOTEIO => "EREMOTEIO",
            Self::EDQUOT => "EDQUOT",
            Self::ENOMEDIUM => "ENOMEDIUM",
            Self::EMEDIUMTYPE => "EMEDIUMTYPE",
            Self::ECANCELED => "ECANCELED",
            Self::ENOKEY => "ENOKEY",
            Self::EKEYEXPIRED => "EKEYEXPIRED",
            Self::EKEYREVOKED => "EKEYREVOKED",
            Self::EKEYREJECTED => "EKEYREJECTED",
            Self::EOWNERDEAD => "EOWNERDEAD",
            Self::ENOTRECOVERABLE => "ENOTRECOVERABLE",
            Self::ERFKILL => "ERFKILL",
            Self::EHWPOISON => "EHWPOISON",
            Self::Unknown => "EUNKNOWN",
        }
    }

    /// Human-readable description matching the POSIX/Linux strerror text.
    pub fn description(self) -> &'static str {
        match self {
            Self::EPERM => "Operation not permitted",
            Self::ENOENT => "No such file or directory",
            Self::ESRCH => "No such process",
            Self::EINTR => "Interrupted system call",
            Self::EIO => "Input/output error",
            Self::ENXIO => "No such device or address",
            Self::E2BIG => "Argument list too long",
            Self::ENOEXEC => "Exec format error",
            Self::EBADF => "Bad file descriptor",
            Self::ECHILD => "No child processes",
            Self::EAGAIN => "Resource temporarily unavailable",
            Self::ENOMEM => "Cannot allocate memory",
            Self::EACCES => "Permission denied",
            Self::EFAULT => "Bad address",
            Self::ENOTBLK => "Block device required",
            Self::EBUSY => "Device or resource busy",
            Self::EEXIST => "File exists",
            Self::EXDEV => "Invalid cross-device link",
            Self::ENODEV => "No such device",
            Self::ENOTDIR => "Not a directory",
            Self::EISDIR => "Is a directory",
            Self::EINVAL => "Invalid argument",
            Self::ENFILE => "Too many open files in system",
            Self::EMFILE => "Too many open files",
            Self::ENOTTY => "Inappropriate ioctl for device",
            Self::ETXTBSY => "Text file busy",
            Self::EFBIG => "File too large",
            Self::ENOSPC => "No space left on device",
            Self::ESPIPE => "Illegal seek",
            Self::EROFS => "Read-only file system",
            Self::EMLINK => "Too many links",
            Self::EPIPE => "Broken pipe",
            Self::EDOM => "Numerical argument out of domain",
            Self::ERANGE => "Numerical result out of range",
            Self::EDEADLK => "Resource deadlock avoided",
            Self::ENAMETOOLONG => "File name too long",
            Self::ENOLCK => "No locks available",
            Self::ENOSYS => "Function not implemented",
            Self::ENOTEMPTY => "Directory not empty",
            Self::ELOOP => "Too many levels of symbolic links",
            Self::Reserved41 => "Reserved",
            Self::ENOMSG => "No message of desired type",
            Self::EIDRM => "Identifier removed",
            Self::ECHRNG => "Channel number out of range",
            Self::EL2NSYNC => "Level 2 not synchronised",
            Self::EL3HLT => "Level 3 halted",
            Self::EL3RST => "Level 3 reset",
            Self::ELNRNG => "Link number out of range",
            Self::EUNATCH => "Protocol driver not attached",
            Self::ENOCSI => "No CSI structure available",
            Self::EL2HLT => "Level 2 halted",
            Self::EBADE => "Invalid exchange",
            Self::EBADR => "Invalid request descriptor",
            Self::EXFULL => "Exchange full",
            Self::ENOANO => "No anode",
            Self::EBADRQC => "Invalid request code",
            Self::EBADSLT => "Invalid slot",
            Self::Reserved58 => "Reserved",
            Self::EBFONT => "Bad font file format",
            Self::ENOSTR => "Device not a stream",
            Self::ENODATA => "No data available",
            Self::ETIME => "Timer expired",
            Self::ENOSR => "Out of streams resources",
            Self::ENONET => "Machine is not on the network",
            Self::ENOPKG => "Package not installed",
            Self::EREMOTE => "Object is remote",
            Self::ENOLINK => "Link has been severed",
            Self::EADV => "Advertise error",
            Self::ESRMNT => "Srmount error",
            Self::ECOMM => "Communication error on send",
            Self::EPROTO => "Protocol error",
            Self::EMULTIHOP => "Multihop attempted",
            Self::EDOTDOT => "RFS specific error",
            Self::EBADMSG => "Bad message",
            Self::EOVERFLOW => "Value too large for defined data type",
            Self::ENOTUNIQ => "Name not unique on network",
            Self::EBADFD => "File descriptor in bad state",
            Self::EREMCHG => "Remote address changed",
            Self::ELIBACC => "Cannot access a needed shared library",
            Self::ELIBBAD => "Accessing a corrupted shared library",
            Self::ELIBSCN => ".lib section in a.out corrupted",
            Self::ELIBMAX => "Attempting to link in too many shared libraries",
            Self::ELIBEXEC => "Cannot exec a shared library directly",
            Self::EILSEQ => "Invalid or incomplete multibyte or wide character",
            Self::ERESTART => "Interrupted syscall should be restarted",
            Self::ESTRPIPE => "Streams pipe error",
            Self::EUSERS => "Too many users",
            Self::ENOTSOCK => "Socket operation on non-socket",
            Self::EDESTADDRREQ => "Destination address required",
            Self::EMSGSIZE => "Message too long",
            Self::EPROTOTYPE => "Protocol wrong type for socket",
            Self::ENOPROTOOPT => "Protocol not available",
            Self::EPROTONOSUPPORT => "Protocol not supported",
            Self::ESOCKTNOSUPPORT => "Socket type not supported",
            Self::EOPNOTSUPP => "Operation not supported",
            Self::EPFNOSUPPORT => "Protocol family not supported",
            Self::EAFNOSUPPORT => "Address family not supported by protocol",
            Self::EADDRINUSE => "Address already in use",
            Self::EADDRNOTAVAIL => "Cannot assign requested address",
            Self::ENETDOWN => "Network is down",
            Self::ENETUNREACH => "Network is unreachable",
            Self::ENETRESET => "Network dropped connection on reset",
            Self::ECONNABORTED => "Software caused connection abort",
            Self::ECONNRESET => "Connection reset by peer",
            Self::ENOBUFS => "No buffer space available",
            Self::EISCONN => "Transport endpoint is already connected",
            Self::ENOTCONN => "Transport endpoint is not connected",
            Self::ESHUTDOWN => "Cannot send after transport endpoint shutdown",
            Self::ETOOMANYREFS => "Too many references: cannot splice",
            Self::ETIMEDOUT => "Connection timed out",
            Self::ECONNREFUSED => "Connection refused",
            Self::EHOSTDOWN => "Host is down",
            Self::EHOSTUNREACH => "No route to host",
            Self::EALREADY => "Operation already in progress",
            Self::EINPROGRESS => "Operation now in progress",
            Self::ESTALE => "Stale file handle",
            Self::EUCLEAN => "Structure needs cleaning",
            Self::ENOTNAM => "Not a XENIX named type file",
            Self::ENAVAIL => "No XENIX semaphores available",
            Self::EISNAM => "Is a named type file",
            Self::EREMOTEIO => "Remote I/O error",
            Self::EDQUOT => "Disk quota exceeded",
            Self::ENOMEDIUM => "No medium found",
            Self::EMEDIUMTYPE => "Wrong medium type",
            Self::ECANCELED => "Operation cancelled",
            Self::ENOKEY => "Required key not available",
            Self::EKEYEXPIRED => "Key has expired",
            Self::EKEYREVOKED => "Key has been revoked",
            Self::EKEYREJECTED => "Key was rejected by service",
            Self::EOWNERDEAD => "Owner died",
            Self::ENOTRECOVERABLE => "State not recoverable",
            Self::ERFKILL => "Operation not possible due to RF-kill",
            Self::EHWPOISON => "Memory page has hardware error",
            Self::Unknown => "Unknown error",
        }
    }
}

impl From<i32> for PosixError {
    fn from(errno: i32) -> Self {
        Self::from_errno(errno)
    }
}

impl From<PosixError> for i32 {
    fn from(err: PosixError) -> Self {
        err as i32
    }
}

impl fmt::Display for PosixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}): {}",
            self.name(),
            *self as i32,
            self.description()
        )
    }
}

pub enum Fault {
    Hard,
    MemManage,
    Bus,
    Usage,
}

pub type Result<T> = core::result::Result<T, PosixError>;

/// IRQ handler signature: `(ctx, vector, userdata)`.
pub type IrqHandler = fn(*mut u8, usize, Option<usize>);

/// Registration callback the kernel hands to the HAL during init.
pub type IrqRegister = fn(usize, IrqHandler, Option<usize>) -> Result<()>;

pub trait Machinelike {
    fn init();
    /// Register HAL-owned IRQs through the kernel-supplied callback.
    fn init_irqs(register: IrqRegister);

    fn print(s: &str) -> Result<()>;

    fn bench_start();
    fn bench_end() -> (u32, f32);

    fn monotonic_now() -> u64;
    fn monotonic_freq() -> u64;
    // Returns the frequency of the machine's systick timer in Hz.
    fn systick_freq() -> u64;

    type ExcepBacktrace: Display;
    type ExcepStackFrame: Display;
    fn backtrace(initial_fp: *const usize, stack_ptr: *const usize) -> Self::ExcepBacktrace;
    fn stack_frame(stack_ptr: *const usize) -> Self::ExcepStackFrame;

    type FaultStatus: Display;
    fn get_fault_status(fault: Fault) -> Self::FaultStatus;

    fn panic_handler(info: &core::panic::PanicInfo) -> !;
    fn do_tick();
}

pub trait Schedable {
    fn trigger_reschedule();
}
