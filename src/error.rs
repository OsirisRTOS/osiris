/// Unix error codes enum covering all standard errno values
/// Values are stored as negative integers matching kernel return values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum UnixError {
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
    /// Try again
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
    /// Resource deadlock would occur
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
    /// Operation not supported on transport endpoint
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

impl UnixError {
    /// Convert to errno value (returns the stored negative value)
    #[inline]
    pub fn to_errno(&self) -> i32 {
        *self as i32
    }
}

impl From<UnixError> for i32 {
    fn from(err: UnixError) -> i32 {
        err.to_errno()
    }
}

impl TryFrom<i32> for UnixError {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        // Only accept negative values (syscall error returns)
        if value >= 0 {
            return Err(());
        }

        match value {
            -1 => Ok(UnixError::EPERM),
            -2 => Ok(UnixError::ENOENT),
            -3 => Ok(UnixError::ESRCH),
            -4 => Ok(UnixError::EINTR),
            -5 => Ok(UnixError::EIO),
            -6 => Ok(UnixError::ENXIO),
            -7 => Ok(UnixError::E2BIG),
            -8 => Ok(UnixError::ENOEXEC),
            -9 => Ok(UnixError::EBADF),
            -10 => Ok(UnixError::ECHILD),
            -11 => Ok(UnixError::EAGAIN),
            -12 => Ok(UnixError::ENOMEM),
            -13 => Ok(UnixError::EACCES),
            -14 => Ok(UnixError::EFAULT),
            -15 => Ok(UnixError::ENOTBLK),
            -16 => Ok(UnixError::EBUSY),
            -17 => Ok(UnixError::EEXIST),
            -18 => Ok(UnixError::EXDEV),
            -19 => Ok(UnixError::ENODEV),
            -20 => Ok(UnixError::ENOTDIR),
            -21 => Ok(UnixError::EISDIR),
            -22 => Ok(UnixError::EINVAL),
            -23 => Ok(UnixError::ENFILE),
            -24 => Ok(UnixError::EMFILE),
            -25 => Ok(UnixError::ENOTTY),
            -26 => Ok(UnixError::ETXTBSY),
            -27 => Ok(UnixError::EFBIG),
            -28 => Ok(UnixError::ENOSPC),
            -29 => Ok(UnixError::ESPIPE),
            -30 => Ok(UnixError::EROFS),
            -31 => Ok(UnixError::EMLINK),
            -32 => Ok(UnixError::EPIPE),
            -33 => Ok(UnixError::EDOM),
            -34 => Ok(UnixError::ERANGE),
            -35 => Ok(UnixError::EDEADLK),
            -36 => Ok(UnixError::ENAMETOOLONG),
            -37 => Ok(UnixError::ENOLCK),
            -38 => Ok(UnixError::ENOSYS),
            -39 => Ok(UnixError::ENOTEMPTY),
            -40 => Ok(UnixError::ELOOP),
            -42 => Ok(UnixError::ENOMSG),
            -43 => Ok(UnixError::EIDRM),
            -44 => Ok(UnixError::ECHRNG),
            -45 => Ok(UnixError::EL2NSYNC),
            -46 => Ok(UnixError::EL3HLT),
            -47 => Ok(UnixError::EL3RST),
            -48 => Ok(UnixError::ELNRNG),
            -49 => Ok(UnixError::EUNATCH),
            -50 => Ok(UnixError::ENOCSI),
            -51 => Ok(UnixError::EL2HLT),
            -52 => Ok(UnixError::EBADE),
            -53 => Ok(UnixError::EBADR),
            -54 => Ok(UnixError::EXFULL),
            -55 => Ok(UnixError::ENOANO),
            -56 => Ok(UnixError::EBADRQC),
            -57 => Ok(UnixError::EBADSLT),
            -59 => Ok(UnixError::EBFONT),
            -60 => Ok(UnixError::ENOSTR),
            -61 => Ok(UnixError::ENODATA),
            -62 => Ok(UnixError::ETIME),
            -63 => Ok(UnixError::ENOSR),
            -64 => Ok(UnixError::ENONET),
            -65 => Ok(UnixError::ENOPKG),
            -66 => Ok(UnixError::EREMOTE),
            -67 => Ok(UnixError::ENOLINK),
            -68 => Ok(UnixError::EADV),
            -69 => Ok(UnixError::ESRMNT),
            -70 => Ok(UnixError::ECOMM),
            -71 => Ok(UnixError::EPROTO),
            -72 => Ok(UnixError::EMULTIHOP),
            -73 => Ok(UnixError::EDOTDOT),
            -74 => Ok(UnixError::EBADMSG),
            -75 => Ok(UnixError::EOVERFLOW),
            -76 => Ok(UnixError::ENOTUNIQ),
            -77 => Ok(UnixError::EBADFD),
            -78 => Ok(UnixError::EREMCHG),
            -79 => Ok(UnixError::ELIBACC),
            -80 => Ok(UnixError::ELIBBAD),
            -81 => Ok(UnixError::ELIBSCN),
            -82 => Ok(UnixError::ELIBMAX),
            -83 => Ok(UnixError::ELIBEXEC),
            -84 => Ok(UnixError::EILSEQ),
            -85 => Ok(UnixError::ERESTART),
            -86 => Ok(UnixError::ESTRPIPE),
            -87 => Ok(UnixError::EUSERS),
            -88 => Ok(UnixError::ENOTSOCK),
            -89 => Ok(UnixError::EDESTADDRREQ),
            -90 => Ok(UnixError::EMSGSIZE),
            -91 => Ok(UnixError::EPROTOTYPE),
            -92 => Ok(UnixError::ENOPROTOOPT),
            -93 => Ok(UnixError::EPROTONOSUPPORT),
            -94 => Ok(UnixError::ESOCKTNOSUPPORT),
            -95 => Ok(UnixError::EOPNOTSUPP),
            -96 => Ok(UnixError::EPFNOSUPPORT),
            -97 => Ok(UnixError::EAFNOSUPPORT),
            -98 => Ok(UnixError::EADDRINUSE),
            -99 => Ok(UnixError::EADDRNOTAVAIL),
            -100 => Ok(UnixError::ENETDOWN),
            -101 => Ok(UnixError::ENETUNREACH),
            -102 => Ok(UnixError::ENETRESET),
            -103 => Ok(UnixError::ECONNABORTED),
            -104 => Ok(UnixError::ECONNRESET),
            -105 => Ok(UnixError::ENOBUFS),
            -106 => Ok(UnixError::EISCONN),
            -107 => Ok(UnixError::ENOTCONN),
            -108 => Ok(UnixError::ESHUTDOWN),
            -109 => Ok(UnixError::ETOOMANYREFS),
            -110 => Ok(UnixError::ETIMEDOUT),
            -111 => Ok(UnixError::ECONNREFUSED),
            -112 => Ok(UnixError::EHOSTDOWN),
            -113 => Ok(UnixError::EHOSTUNREACH),
            -114 => Ok(UnixError::EALREADY),
            -115 => Ok(UnixError::EINPROGRESS),
            -116 => Ok(UnixError::ESTALE),
            -117 => Ok(UnixError::EUCLEAN),
            -118 => Ok(UnixError::ENOTNAM),
            -119 => Ok(UnixError::ENAVAIL),
            -120 => Ok(UnixError::EISNAM),
            -121 => Ok(UnixError::EREMOTEIO),
            -122 => Ok(UnixError::EDQUOT),
            -123 => Ok(UnixError::ENOMEDIUM),
            -124 => Ok(UnixError::EMEDIUMTYPE),
            -125 => Ok(UnixError::ECANCELED),
            -126 => Ok(UnixError::ENOKEY),
            -127 => Ok(UnixError::EKEYEXPIRED),
            -128 => Ok(UnixError::EKEYREVOKED),
            -129 => Ok(UnixError::EKEYREJECTED),
            -130 => Ok(UnixError::EOWNERDEAD),
            -131 => Ok(UnixError::ENOTRECOVERABLE),
            -132 => Ok(UnixError::ERFKILL),
            -133 => Ok(UnixError::EHWPOISON),
            _ => Err(()),
        }
    }
}

impl TryFrom<isize> for UnixError {
    type Error = ();

    fn try_from(value: isize) -> Result<Self, Self::Error> {
        // Only accept negative values (syscall error returns)
        if value >= 0 {
            return Err(());
        }

        match value {
            -1 => Ok(UnixError::EPERM),
            -2 => Ok(UnixError::ENOENT),
            -3 => Ok(UnixError::ESRCH),
            -4 => Ok(UnixError::EINTR),
            -5 => Ok(UnixError::EIO),
            -6 => Ok(UnixError::ENXIO),
            -7 => Ok(UnixError::E2BIG),
            -8 => Ok(UnixError::ENOEXEC),
            -9 => Ok(UnixError::EBADF),
            -10 => Ok(UnixError::ECHILD),
            -11 => Ok(UnixError::EAGAIN),
            -12 => Ok(UnixError::ENOMEM),
            -13 => Ok(UnixError::EACCES),
            -14 => Ok(UnixError::EFAULT),
            -15 => Ok(UnixError::ENOTBLK),
            -16 => Ok(UnixError::EBUSY),
            -17 => Ok(UnixError::EEXIST),
            -18 => Ok(UnixError::EXDEV),
            -19 => Ok(UnixError::ENODEV),
            -20 => Ok(UnixError::ENOTDIR),
            -21 => Ok(UnixError::EISDIR),
            -22 => Ok(UnixError::EINVAL),
            -23 => Ok(UnixError::ENFILE),
            -24 => Ok(UnixError::EMFILE),
            -25 => Ok(UnixError::ENOTTY),
            -26 => Ok(UnixError::ETXTBSY),
            -27 => Ok(UnixError::EFBIG),
            -28 => Ok(UnixError::ENOSPC),
            -29 => Ok(UnixError::ESPIPE),
            -30 => Ok(UnixError::EROFS),
            -31 => Ok(UnixError::EMLINK),
            -32 => Ok(UnixError::EPIPE),
            -33 => Ok(UnixError::EDOM),
            -34 => Ok(UnixError::ERANGE),
            -35 => Ok(UnixError::EDEADLK),
            -36 => Ok(UnixError::ENAMETOOLONG),
            -37 => Ok(UnixError::ENOLCK),
            -38 => Ok(UnixError::ENOSYS),
            -39 => Ok(UnixError::ENOTEMPTY),
            -40 => Ok(UnixError::ELOOP),
            -42 => Ok(UnixError::ENOMSG),
            -43 => Ok(UnixError::EIDRM),
            -44 => Ok(UnixError::ECHRNG),
            -45 => Ok(UnixError::EL2NSYNC),
            -46 => Ok(UnixError::EL3HLT),
            -47 => Ok(UnixError::EL3RST),
            -48 => Ok(UnixError::ELNRNG),
            -49 => Ok(UnixError::EUNATCH),
            -50 => Ok(UnixError::ENOCSI),
            -51 => Ok(UnixError::EL2HLT),
            -52 => Ok(UnixError::EBADE),
            -53 => Ok(UnixError::EBADR),
            -54 => Ok(UnixError::EXFULL),
            -55 => Ok(UnixError::ENOANO),
            -56 => Ok(UnixError::EBADRQC),
            -57 => Ok(UnixError::EBADSLT),
            -59 => Ok(UnixError::EBFONT),
            -60 => Ok(UnixError::ENOSTR),
            -61 => Ok(UnixError::ENODATA),
            -62 => Ok(UnixError::ETIME),
            -63 => Ok(UnixError::ENOSR),
            -64 => Ok(UnixError::ENONET),
            -65 => Ok(UnixError::ENOPKG),
            -66 => Ok(UnixError::EREMOTE),
            -67 => Ok(UnixError::ENOLINK),
            -68 => Ok(UnixError::EADV),
            -69 => Ok(UnixError::ESRMNT),
            -70 => Ok(UnixError::ECOMM),
            -71 => Ok(UnixError::EPROTO),
            -72 => Ok(UnixError::EMULTIHOP),
            -73 => Ok(UnixError::EDOTDOT),
            -74 => Ok(UnixError::EBADMSG),
            -75 => Ok(UnixError::EOVERFLOW),
            -76 => Ok(UnixError::ENOTUNIQ),
            -77 => Ok(UnixError::EBADFD),
            -78 => Ok(UnixError::EREMCHG),
            -79 => Ok(UnixError::ELIBACC),
            -80 => Ok(UnixError::ELIBBAD),
            -81 => Ok(UnixError::ELIBSCN),
            -82 => Ok(UnixError::ELIBMAX),
            -83 => Ok(UnixError::ELIBEXEC),
            -84 => Ok(UnixError::EILSEQ),
            -85 => Ok(UnixError::ERESTART),
            -86 => Ok(UnixError::ESTRPIPE),
            -87 => Ok(UnixError::EUSERS),
            -88 => Ok(UnixError::ENOTSOCK),
            -89 => Ok(UnixError::EDESTADDRREQ),
            -90 => Ok(UnixError::EMSGSIZE),
            -91 => Ok(UnixError::EPROTOTYPE),
            -92 => Ok(UnixError::ENOPROTOOPT),
            -93 => Ok(UnixError::EPROTONOSUPPORT),
            -94 => Ok(UnixError::ESOCKTNOSUPPORT),
            -95 => Ok(UnixError::EOPNOTSUPP),
            -96 => Ok(UnixError::EPFNOSUPPORT),
            -97 => Ok(UnixError::EAFNOSUPPORT),
            -98 => Ok(UnixError::EADDRINUSE),
            -99 => Ok(UnixError::EADDRNOTAVAIL),
            -100 => Ok(UnixError::ENETDOWN),
            -101 => Ok(UnixError::ENETUNREACH),
            -102 => Ok(UnixError::ENETRESET),
            -103 => Ok(UnixError::ECONNABORTED),
            -104 => Ok(UnixError::ECONNRESET),
            -105 => Ok(UnixError::ENOBUFS),
            -106 => Ok(UnixError::EISCONN),
            -107 => Ok(UnixError::ENOTCONN),
            -108 => Ok(UnixError::ESHUTDOWN),
            -109 => Ok(UnixError::ETOOMANYREFS),
            -110 => Ok(UnixError::ETIMEDOUT),
            -111 => Ok(UnixError::ECONNREFUSED),
            -112 => Ok(UnixError::EHOSTDOWN),
            -113 => Ok(UnixError::EHOSTUNREACH),
            -114 => Ok(UnixError::EALREADY),
            -115 => Ok(UnixError::EINPROGRESS),
            -116 => Ok(UnixError::ESTALE),
            -117 => Ok(UnixError::EUCLEAN),
            -118 => Ok(UnixError::ENOTNAM),
            -119 => Ok(UnixError::ENAVAIL),
            -120 => Ok(UnixError::EISNAM),
            -121 => Ok(UnixError::EREMOTEIO),
            -122 => Ok(UnixError::EDQUOT),
            -123 => Ok(UnixError::ENOMEDIUM),
            -124 => Ok(UnixError::EMEDIUMTYPE),
            -125 => Ok(UnixError::ECANCELED),
            -126 => Ok(UnixError::ENOKEY),
            -127 => Ok(UnixError::EKEYEXPIRED),
            -128 => Ok(UnixError::EKEYREVOKED),
            -129 => Ok(UnixError::EKEYREJECTED),
            -130 => Ok(UnixError::EOWNERDEAD),
            -131 => Ok(UnixError::ENOTRECOVERABLE),
            -132 => Ok(UnixError::ERFKILL),
            -133 => Ok(UnixError::EHWPOISON),
            _ => Err(()),
        }
    }
}