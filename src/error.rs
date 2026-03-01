/// Posix error codes enum covering all standard errno values.
/// Values are stored as negative integers matching kernel return values.
///
/// Source: Linux kernel include/uapi/asm-generic/errno-base.h (1–34)
///         and include/uapi/asm-generic/errno.h (35–133).
///         Valid for x86 (32/64-bit) and ARM (32/64-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i16)]
pub enum PosixError {
    /// Operation not permitted
    EPERM = -1,
    /// No such file or directory
    ENOENT = -2,
    /// No such process
    ESRCH = -3,
    /// Interrupted system call
    EINTR = -4,
    /// I/O error
    EIO = -5,
    /// No such device or address
    ENXIO = -6,
    /// Argument list too long
    E2BIG = -7,
    /// Exec format error
    ENOEXEC = -8,
    /// Bad file number
    EBADF = -9,
    /// No child processes
    ECHILD = -10,
    /// Try again (EWOULDBLOCK is an alias for this value)
    EAGAIN = -11,
    /// Out of memory
    ENOMEM = -12,
    /// Permission denied
    EACCES = -13,
    /// Bad address
    EFAULT = -14,
    /// Block device required
    ENOTBLK = -15,
    /// Device or resource busy
    EBUSY = -16,
    /// File exists
    EEXIST = -17,
    /// Cross-device link
    EXDEV = -18,
    /// No such device
    ENODEV = -19,
    /// Not a directory
    ENOTDIR = -20,
    /// Is a directory
    EISDIR = -21,
    /// Invalid argument
    EINVAL = -22,
    /// File table overflow
    ENFILE = -23,
    /// Too many open files
    EMFILE = -24,
    /// Not a typewriter
    ENOTTY = -25,
    /// Text file busy
    ETXTBSY = -26,
    /// File too large
    EFBIG = -27,
    /// No space left on device
    ENOSPC = -28,
    /// Illegal seek
    ESPIPE = -29,
    /// Read-only file system
    EROFS = -30,
    /// Too many links
    EMLINK = -31,
    /// Broken pipe
    EPIPE = -32,
    /// Math argument out of domain of func
    EDOM = -33,
    /// Math result not representable
    ERANGE = -34,
    /// Resource deadlock would occur (EDEADLOCK is an alias for this value)
    EDEADLK = -35,
    /// File name too long
    ENAMETOOLONG = -36,
    /// No record locks available
    ENOLCK = -37,
    /// Function not implemented
    ENOSYS = -38,
    /// Directory not empty
    ENOTEMPTY = -39,
    /// Too many symbolic links encountered
    ELOOP = -40,
    /// Reserved (EWOULDBLOCK alias slot, use EAGAIN)
    _RESERVED_41 = -41,
    /// No message of desired type
    ENOMSG = -42,
    /// Identifier removed
    EIDRM = -43,
    /// Channel number out of range
    ECHRNG = -44,
    /// Level 2 not synchronized
    EL2NSYNC = -45,
    /// Level 3 halted
    EL3HLT = -46,
    /// Level 3 reset
    EL3RST = -47,
    /// Link number out of range
    ELNRNG = -48,
    /// Protocol driver not attached
    EUNATCH = -49,
    /// No CSI structure available
    ENOCSI = -50,
    /// Level 2 halted
    EL2HLT = -51,
    /// Invalid exchange
    EBADE = -52,
    /// Invalid request descriptor
    EBADR = -53,
    /// Exchange full
    EXFULL = -54,
    /// No anode
    ENOANO = -55,
    /// Invalid request code
    EBADRQC = -56,
    /// Invalid slot
    EBADSLT = -57,
    /// Reserved (EDEADLOCK alias slot, use EDEADLK)
    _RESERVED_58 = -58,
    /// Bad font file format
    EBFONT = -59,
    /// Device not a stream
    ENOSTR = -60,
    /// No data available
    ENODATA = -61,
    /// Timer expired
    ETIME = -62,
    /// Out of streams resources
    ENOSR = -63,
    /// Machine is not on the network
    ENONET = -64,
    /// Package not installed
    ENOPKG = -65,
    /// Object is remote
    EREMOTE = -66,
    /// Link has been severed
    ENOLINK = -67,
    /// Advertise error
    EADV = -68,
    /// Srmount error
    ESRMNT = -69,
    /// Communication error on send
    ECOMM = -70,
    /// Protocol error
    EPROTO = -71,
    /// Multihop attempted
    EMULTIHOP = -72,
    /// RFS specific error
    EDOTDOT = -73,
    /// Not a data message
    EBADMSG = -74,
    /// Value too large for defined data type
    EOVERFLOW = -75,
    /// Name not unique on network
    ENOTUNIQ = -76,
    /// File descriptor in bad state
    EBADFD = -77,
    /// Remote address changed
    EREMCHG = -78,
    /// Can not access a needed shared library
    ELIBACC = -79,
    /// Accessing a corrupted shared library
    ELIBBAD = -80,
    /// .lib section in a.out corrupted
    ELIBSCN = -81,
    /// Attempting to link in too many shared libraries
    ELIBMAX = -82,
    /// Cannot exec a shared library directly
    ELIBEXEC = -83,
    /// Illegal byte sequence
    EILSEQ = -84,
    /// Interrupted system call should be restarted
    ERESTART = -85,
    /// Streams pipe error
    ESTRPIPE = -86,
    /// Too many users
    EUSERS = -87,
    /// Socket operation on non-socket
    ENOTSOCK = -88,
    /// Destination address required
    EDESTADDRREQ = -89,
    /// Message too long
    EMSGSIZE = -90,
    /// Protocol wrong type for socket
    EPROTOTYPE = -91,
    /// Protocol not available
    ENOPROTOOPT = -92,
    /// Protocol not supported
    EPROTONOSUPPORT = -93,
    /// Socket type not supported
    ESOCKTNOSUPPORT = -94,
    /// Operation not supported on transport endpoint (ENOTSUP is an alias for this value)
    EOPNOTSUPP = -95,
    /// Protocol family not supported
    EPFNOSUPPORT = -96,
    /// Address family not supported by protocol
    EAFNOSUPPORT = -97,
    /// Address already in use
    EADDRINUSE = -98,
    /// Cannot assign requested address
    EADDRNOTAVAIL = -99,
    /// Network is down
    ENETDOWN = -100,
    /// Network is unreachable
    ENETUNREACH = -101,
    /// Network dropped connection because of reset
    ENETRESET = -102,
    /// Software caused connection abort
    ECONNABORTED = -103,
    /// Connection reset by peer
    ECONNRESET = -104,
    /// No buffer space available
    ENOBUFS = -105,
    /// Transport endpoint is already connected
    EISCONN = -106,
    /// Transport endpoint is not connected
    ENOTCONN = -107,
    /// Cannot send after transport endpoint shutdown
    ESHUTDOWN = -108,
    /// Too many references: cannot splice
    ETOOMANYREFS = -109,
    /// Connection timed out
    ETIMEDOUT = -110,
    /// Connection refused
    ECONNREFUSED = -111,
    /// Host is down
    EHOSTDOWN = -112,
    /// No route to host
    EHOSTUNREACH = -113,
    /// Operation already in progress
    EALREADY = -114,
    /// Operation now in progress
    EINPROGRESS = -115,
    /// Stale file handle
    ESTALE = -116,
    /// Structure needs cleaning
    EUCLEAN = -117,
    /// Not a XENIX named type file
    ENOTNAM = -118,
    /// No XENIX semaphores available
    ENAVAIL = -119,
    /// Is a named type file
    EISNAM = -120,
    /// Remote I/O error
    EREMOTEIO = -121,
    /// Quota exceeded
    EDQUOT = -122,
    /// No medium found
    ENOMEDIUM = -123,
    /// Wrong medium type
    EMEDIUMTYPE = -124,
    /// Operation Canceled
    ECANCELED = -125,
    /// Required key not available
    ENOKEY = -126,
    /// Key has expired
    EKEYEXPIRED = -127,
    /// Key has been revoked
    EKEYREVOKED = -128,
    /// Key was rejected by service
    EKEYREJECTED = -129,
    /// Owner died
    EOWNERDEAD = -130,
    /// State not recoverable
    ENOTRECOVERABLE = -131,
    /// Operation not possible due to RF-kill
    ERFKILL = -132,
    /// Memory page has hardware error
    EHWPOISON = -133,
}

/// Convenience alias: EWOULDBLOCK is EAGAIN in Linux
pub const EWOULDBLOCK: PosixError = PosixError::EAGAIN;
/// Convenience alias: EDEADLOCK is EDEADLK in Linux
pub const EDEADLOCK: PosixError = PosixError::EDEADLK;
/// Convenience alias: ENOTSUP is EOPNOTSUPP in Linux
pub const ENOTSUP: PosixError = PosixError::EOPNOTSUPP;

// ── Inherent methods ─────────────────────────────────────────────────

impl PosixError {
    /// Bounds for Try Impls: The bounds are both inclusive
    const MIN: i16 = -133;
    const MAX: i16 = -1;

    #[inline]
    pub fn to_errno(&self) -> i16 {
        *self as i16
    }
}

// ── Base conversion: i16 (matches #[repr(i16)]) ─────────────────────

impl From<PosixError> for i16 {
    #[inline]
    fn from(err: PosixError) -> i16 {
        err.to_errno()
    }
}

impl TryFrom<i16> for PosixError {
    type Error = ();

    #[inline]
    fn try_from(value: i16) -> Result<Self, Self::Error> {
        if value < Self::MIN || value > Self::MAX {
            return Err(());
        }
        // SAFETY: 
        // The range of error values is padded with reserved fields to be continous, so every value in the range is a valid PosixError variant
        // The bounds are validated against the provided MIN...MAX range
        Ok(unsafe { core::mem::transmute(value) })
    }
}

// ── Widening conversions built on the i16 base ──────────────────────

impl From<PosixError> for i32 {
    #[inline]
    fn from(err: PosixError) -> i32 {
        err.to_errno() as i32
    }
}

impl TryFrom<i32> for PosixError {
    type Error = ();

    #[inline]
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        let narrow = i16::try_from(value).map_err(|_| ())?;
        Self::try_from(narrow)
    }
}

impl From<PosixError> for isize {
    #[inline]
    fn from(err: PosixError) -> isize {
        err.to_errno() as isize
    }
}

impl TryFrom<isize> for PosixError {
    type Error = ();

    #[inline]
    fn try_from(value: isize) -> Result<Self, Self::Error> {
        let narrow = i16::try_from(value).map_err(|_| ())?;
        Self::try_from(narrow)
    }
}

impl From<PosixError> for usize {
    #[inline]
    fn from(err: PosixError) -> usize {
        err.to_errno() as usize
    }
}