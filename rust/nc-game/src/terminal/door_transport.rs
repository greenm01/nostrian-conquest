use std::io::{self, Read, Write};

#[cfg(windows)]
use std::io::IsTerminal;
#[cfg(windows)]
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DoorTransport {
    #[default]
    Stdio,
    TcpConnect {
        host: &'static str,
        port: u16,
    },
    #[cfg(windows)]
    SocketDescriptor {
        descriptor: u64,
    },
}

pub(crate) trait DoorIo: Read + Write {
    fn wait_ready(&mut self, timeout_ms: i32) -> io::Result<bool>;
}

pub(crate) fn build_door_io(transport: DoorTransport) -> io::Result<Box<dyn DoorIo>> {
    match transport {
        DoorTransport::Stdio => Ok(Box::new(StdioDoorIo::new())),
        DoorTransport::TcpConnect { host, port } => {
            Ok(Box::new(SocketDoorIo::connect(host, port)?))
        }
        #[cfg(windows)]
        DoorTransport::SocketDescriptor { descriptor } => {
            Ok(Box::new(SocketDoorIo::from_socket_descriptor(descriptor)?))
        }
    }
}

struct StdioDoorIo {
    input: RawStdin,
    output: io::Stdout,
    /// The console input mode to restore on exit, when VT input was turned on.
    #[cfg(windows)]
    restore_console_mode: Option<u32>,
}

impl StdioDoorIo {
    fn new() -> Self {
        Self {
            input: RawStdin,
            output: io::stdout(),
            #[cfg(windows)]
            restore_console_mode: enable_console_vt_input(),
        }
    }
}

#[cfg(windows)]
impl Drop for StdioDoorIo {
    fn drop(&mut self) {
        if let Some(mode) = self.restore_console_mode {
            let _ = set_console_input_mode(mode);
        }
    }
}

#[cfg(windows)]
unsafe extern "system" {
    fn GetConsoleMode(handle: *mut std::ffi::c_void, mode: *mut u32) -> i32;
    fn SetConsoleMode(handle: *mut std::ffi::c_void, mode: u32) -> i32;
}

// A console reports arrow, Home, End, and page keys as key events without a
// character, which `stdin_ready` discards. VT input mode turns them into the
// sequences a BBS caller's terminal sends (`ESC [ A`). Consoles older than
// Windows 10 refuse the mode, and those keys stay dead there. Returns the
// mode to restore.
#[cfg(windows)]
fn enable_console_vt_input() -> Option<u32> {
    use std::os::windows::io::AsRawHandle;

    const ENABLE_VIRTUAL_TERMINAL_INPUT: u32 = 0x0200;

    let stdin = io::stdin();
    if !stdin.is_terminal() {
        return None;
    }
    let mut mode = 0u32;
    if unsafe { GetConsoleMode(stdin.as_raw_handle() as *mut _, &mut mode) } == 0 {
        return None;
    }
    set_console_input_mode(mode | ENABLE_VIRTUAL_TERMINAL_INPUT).ok()?;
    Some(mode)
}

#[cfg(windows)]
fn set_console_input_mode(mode: u32) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;

    if unsafe { SetConsoleMode(io::stdin().as_raw_handle() as *mut _, mode) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

impl Read for StdioDoorIo {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.input.read(buf)
    }
}

impl Write for StdioDoorIo {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        #[cfg(windows)]
        if self.output.is_terminal() {
            self.output.flush()?;
            return write_console_bytes(buf);
        }
        self.output.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

impl DoorIo for StdioDoorIo {
    fn wait_ready(&mut self, timeout_ms: i32) -> io::Result<bool> {
        stdin_ready(timeout_ms)
    }
}

#[cfg(unix)]
struct RawStdin;

#[cfg(unix)]
impl Read for RawStdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use std::os::fd::AsRawFd;

        unsafe extern "C" {
            fn read(fd: i32, buf: *mut std::ffi::c_void, count: usize) -> isize;
        }

        let fd = io::stdin().as_raw_fd();
        let ret = unsafe { read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }
}

#[cfg(windows)]
struct RawStdin;

#[cfg(windows)]
impl Read for RawStdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut stdin = io::stdin();
        if !stdin.is_terminal() {
            return stdin.read(buf);
        }

        use std::os::windows::io::AsRawHandle;

        unsafe extern "system" {
            fn ReadFile(
                handle: *mut std::ffi::c_void,
                buffer: *mut u8,
                count: u32,
                bytes_read: *mut u32,
                overlapped: *mut std::ffi::c_void,
            ) -> i32;
        }

        let handle = io::stdin().as_raw_handle();
        let mut bytes_read: u32 = 0;
        let len = buf.len().min(u32::MAX as usize) as u32;
        let rc = unsafe {
            ReadFile(
                handle as *mut _,
                buf.as_mut_ptr(),
                len,
                &mut bytes_read,
                std::ptr::null_mut(),
            )
        };
        if rc == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(bytes_read as usize)
        }
    }
}

// Rust's console `Stdout` rejects bytes that are not UTF-8, and the door
// writes CP437. Write the bytes as they are, as `RawStdin` reads them; the
// console shows them in its output code page.
#[cfg(windows)]
fn write_console_bytes(buf: &[u8]) -> io::Result<usize> {
    use std::os::windows::io::AsRawHandle;

    unsafe extern "system" {
        fn WriteFile(
            handle: *mut std::ffi::c_void,
            buffer: *const u8,
            count: u32,
            bytes_written: *mut u32,
            overlapped: *mut std::ffi::c_void,
        ) -> i32;
    }

    let handle = io::stdout().as_raw_handle();
    let mut bytes_written: u32 = 0;
    let len = buf.len().min(u32::MAX as usize) as u32;
    let rc = unsafe {
        WriteFile(
            handle as *mut _,
            buf.as_ptr(),
            len,
            &mut bytes_written,
            std::ptr::null_mut(),
        )
    };
    if rc == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(bytes_written as usize)
    }
}

#[cfg(any(unix, windows))]
struct SocketDoorIo {
    stream: std::net::TcpStream,
}

#[cfg(any(unix, windows))]
impl SocketDoorIo {
    fn connect(host: &str, port: u16) -> io::Result<Self> {
        #[cfg(windows)]
        ensure_winsock_started()?;

        let stream = std::net::TcpStream::connect((host, port)).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("failed to connect to door socket at {host}:{port}: {err}"),
            )
        })?;
        Ok(Self { stream })
    }

    #[cfg(windows)]
    fn from_socket_descriptor(descriptor: u64) -> io::Result<Self> {
        use std::os::windows::io::{FromRawSocket, RawSocket};

        if descriptor == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "DOOR32 socket descriptor must be non-zero",
            ));
        }

        ensure_winsock_started()?;
        let stream = unsafe { std::net::TcpStream::from_raw_socket(descriptor as RawSocket) };
        if let Err(err) = stream.peer_addr() {
            return Err(io::Error::new(
                err.kind(),
                format!("failed to adopt DOOR32 socket descriptor {descriptor}: {err}"),
            ));
        }
        Ok(Self { stream })
    }
}

#[cfg(windows)]
fn ensure_winsock_started() -> io::Result<()> {
    use std::sync::OnceLock;

    #[repr(C)]
    struct WsaData {
        w_version: u16,
        w_high_version: u16,
        sz_description: [u8; 257],
        sz_system_status: [u8; 129],
        i_max_sockets: u16,
        i_max_udp_dg: u16,
        lp_vendor_info: *mut i8,
    }

    unsafe extern "system" {
        fn WSAStartup(version_requested: u16, data: *mut WsaData) -> i32;
    }

    static STARTUP: OnceLock<Result<(), i32>> = OnceLock::new();

    match STARTUP.get_or_init(|| {
        let mut data = WsaData {
            w_version: 0,
            w_high_version: 0,
            sz_description: [0; 257],
            sz_system_status: [0; 129],
            i_max_sockets: 0,
            i_max_udp_dg: 0,
            lp_vendor_info: std::ptr::null_mut(),
        };
        let rc = unsafe { WSAStartup(0x0202, &mut data) };
        if rc == 0 { Ok(()) } else { Err(rc) }
    }) {
        Ok(()) => Ok(()),
        Err(code) => Err(io::Error::from_raw_os_error(*code)),
    }
}

#[cfg(any(unix, windows))]
impl Read for SocketDoorIo {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            match self.stream.read(buf) {
                Ok(size) => return Ok(size),
                Err(err) if socket_would_block(&err) => {
                    socket_wait(&self.stream, SocketWait::Read, -1)?;
                }
                Err(err) => return Err(err),
            }
        }
    }
}

#[cfg(any(unix, windows))]
impl Write for SocketDoorIo {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        loop {
            match self.stream.write(buf) {
                Ok(size) => return Ok(size),
                Err(err) if socket_would_block(&err) => {
                    socket_wait(&self.stream, SocketWait::Write, -1)?;
                }
                Err(err) => return Err(err),
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

#[cfg(any(unix, windows))]
impl DoorIo for SocketDoorIo {
    fn wait_ready(&mut self, timeout_ms: i32) -> io::Result<bool> {
        socket_wait(&self.stream, SocketWait::Read, timeout_ms)
    }
}

/// `POLLERR | POLLHUP | POLLNVAL`: a hung-up or failed fd counts as ready so
/// the following read reports the closure instead of the wait spinning.
#[cfg(unix)]
const POLL_CLOSED: i16 = 0x0008 | 0x0010 | 0x0020;

#[cfg(unix)]
fn stdin_ready(timeout_ms: i32) -> io::Result<bool> {
    use std::os::fd::AsRawFd;

    const POLLIN: i16 = 0x0001;

    #[repr(C)]
    struct PollFd {
        fd: i32,
        events: i16,
        revents: i16,
    }

    unsafe extern "C" {
        fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
    }

    let stdin = io::stdin();
    let mut fds = [PollFd {
        fd: stdin.as_raw_fd(),
        events: POLLIN,
        revents: 0,
    }];

    loop {
        let rc = unsafe { poll(fds.as_mut_ptr(), fds.len(), timeout_ms) };
        if rc >= 0 {
            return Ok(rc > 0 && (fds[0].revents & (POLLIN | POLL_CLOSED)) != 0);
        }
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::Interrupted {
            continue;
        }
        return Err(err);
    }
}

/// One console `INPUT_RECORD`. Every member of its event union is 16 bytes
/// with 4-byte alignment, so the key-event member stands in for the union.
#[cfg_attr(not(windows), allow(dead_code))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ConsoleInputRecord {
    event_type: u16,
    key_event: ConsoleKeyEventRecord,
}

/// `KEY_EVENT_RECORD`, with `uChar` read as its UTF-16 member.
#[cfg_attr(not(windows), allow(dead_code))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ConsoleKeyEventRecord {
    key_down: i32,
    repeat_count: u16,
    virtual_key_code: u16,
    virtual_scan_code: u16,
    unicode_char: u16,
    control_key_state: u32,
}

#[cfg_attr(not(windows), allow(dead_code))]
const CONSOLE_KEY_EVENT: u16 = 0x0001;

/// Whether `ReadFile` on the console returns bytes for this record: a key
/// press that carries a character. Key releases, bare modifier and navigation
/// keys, and mouse, focus, menu, and resize records produce nothing, so they
/// must not wake a door wait whose read would then block.
#[cfg_attr(not(windows), allow(dead_code))]
fn console_record_has_key_input(record: &ConsoleInputRecord) -> bool {
    record.event_type == CONSOLE_KEY_EVENT
        && record.key_event.key_down != 0
        && record.key_event.unicode_char != 0
}

#[cfg(windows)]
fn stdin_ready(timeout_ms: i32) -> io::Result<bool> {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;

    unsafe extern "system" {
        fn PeekNamedPipe(
            handle: *mut c_void,
            buffer: *mut u8,
            buffer_size: u32,
            bytes_read: *mut u32,
            total_bytes_available: *mut u32,
            bytes_left_this_message: *mut u32,
        ) -> i32;
        fn PeekConsoleInputW(
            handle: *mut c_void,
            buffer: *mut ConsoleInputRecord,
            length: u32,
            events_read: *mut u32,
        ) -> i32;
        fn ReadConsoleInputW(
            handle: *mut c_void,
            buffer: *mut ConsoleInputRecord,
            length: u32,
            events_read: *mut u32,
        ) -> i32;
        fn WaitForSingleObject(handle: *mut c_void, millis: u32) -> u32;
    }

    const WAIT_OBJECT_0: u32 = 0;
    const WAIT_TIMEOUT: u32 = 0x102;
    const INFINITE: u32 = 0xFFFFFFFF;

    let stdin = io::stdin();
    let handle = stdin.as_raw_handle();
    let deadline = if timeout_ms < 0 {
        None
    } else {
        Some(Instant::now() + Duration::from_millis(timeout_ms as u64))
    };
    if !stdin.is_terminal() {
        loop {
            let mut total_bytes_available = 0u32;
            let rc = unsafe {
                PeekNamedPipe(
                    handle as *mut _,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut total_bytes_available,
                    std::ptr::null_mut(),
                )
            };
            if rc != 0 {
                return Ok(total_bytes_available > 0);
            }

            let err = io::Error::last_os_error();
            if timeout_ms == 0 {
                return Ok(false);
            }
            if deadline.is_some_and(|value| Instant::now() >= value) {
                return Ok(false);
            }
            if err.kind() == io::ErrorKind::BrokenPipe {
                // Closed pipe: the read reports EOF instead of waiting.
                return Ok(true);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    // The console handle is signaled while any input record is queued, but
    // `ReadFile` skips records without a character and then blocks. Discard
    // those until a usable key press is first in the queue.
    loop {
        let mut record = ConsoleInputRecord::default();
        let mut count = 0u32;
        if unsafe { PeekConsoleInputW(handle as *mut _, &mut record, 1, &mut count) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if count == 0 {
            let millis = deadline.map_or(INFINITE, |deadline| {
                let remaining = deadline.saturating_duration_since(Instant::now());
                remaining.as_millis().min(u128::from(INFINITE - 1)) as u32
            });
            match unsafe { WaitForSingleObject(handle as *mut _, millis) } {
                WAIT_OBJECT_0 => continue,
                WAIT_TIMEOUT => return Ok(false),
                _ => return Err(io::Error::last_os_error()),
            }
        }
        if console_record_has_key_input(&record) {
            return Ok(true);
        }
        if unsafe { ReadConsoleInputW(handle as *mut _, &mut record, 1, &mut count) } == 0 {
            return Err(io::Error::last_os_error());
        }
    }
}

#[cfg(windows)]
enum SocketWait {
    Read,
    Write,
}

#[cfg(unix)]
enum SocketWait {
    Read,
    Write,
}

#[cfg(windows)]
fn socket_wait(
    stream: &std::net::TcpStream,
    direction: SocketWait,
    timeout_ms: i32,
) -> io::Result<bool> {
    use std::ffi::c_long;
    use std::os::windows::io::AsRawSocket;

    #[repr(C)]
    struct FdSet {
        fd_count: u32,
        fd_array: [usize; 64],
    }

    #[repr(C)]
    struct TimeVal {
        tv_sec: c_long,
        tv_usec: c_long,
    }

    unsafe extern "system" {
        fn select(
            nfds: i32,
            readfds: *mut FdSet,
            writefds: *mut FdSet,
            exceptfds: *mut FdSet,
            timeout: *const TimeVal,
        ) -> i32;
        fn WSAGetLastError() -> i32;
    }

    let mut socketfds = FdSet {
        fd_count: 1,
        fd_array: [0; 64],
    };
    socketfds.fd_array[0] = stream.as_raw_socket() as usize;

    let mut timeout = TimeVal {
        tv_sec: 0,
        tv_usec: 0,
    };
    let timeout_ptr = if timeout_ms < 0 {
        std::ptr::null()
    } else {
        timeout.tv_sec = (timeout_ms / 1000) as c_long;
        timeout.tv_usec = ((timeout_ms % 1000) * 1000) as c_long;
        &timeout
    };

    let rc = unsafe {
        select(
            0,
            match direction {
                SocketWait::Read => &mut socketfds,
                SocketWait::Write => std::ptr::null_mut(),
            },
            match direction {
                SocketWait::Read => std::ptr::null_mut(),
                SocketWait::Write => &mut socketfds,
            },
            std::ptr::null_mut(),
            timeout_ptr,
        )
    };
    if rc >= 0 {
        return Ok(rc > 0 && socketfds.fd_count > 0);
    }

    let raw = unsafe { WSAGetLastError() };
    Err(io::Error::from_raw_os_error(raw))
}

#[cfg(windows)]
fn socket_would_block(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock || err.raw_os_error() == Some(10035)
}

#[cfg(unix)]
fn socket_wait(
    stream: &std::net::TcpStream,
    direction: SocketWait,
    timeout_ms: i32,
) -> io::Result<bool> {
    use std::os::fd::AsRawFd;

    const POLLIN: i16 = 0x0001;
    const POLLOUT: i16 = 0x0004;

    #[repr(C)]
    struct PollFd {
        fd: i32,
        events: i16,
        revents: i16,
    }

    unsafe extern "C" {
        fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
    }

    let events = match direction {
        SocketWait::Read => POLLIN,
        SocketWait::Write => POLLOUT,
    };
    let mut fds = [PollFd {
        fd: stream.as_raw_fd(),
        events,
        revents: 0,
    }];

    loop {
        let rc = unsafe { poll(fds.as_mut_ptr(), fds.len(), timeout_ms) };
        if rc >= 0 {
            return Ok(rc > 0 && (fds[0].revents & (events | POLL_CLOSED)) != 0);
        }
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::Interrupted {
            continue;
        }
        return Err(err);
    }
}

#[cfg(unix)]
fn socket_would_block(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
}

#[cfg(not(any(unix, windows)))]
fn stdin_ready(_timeout_ms: i32) -> io::Result<bool> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{ConsoleInputRecord, ConsoleKeyEventRecord, console_record_has_key_input};

    // Inline: the record layout is a private FFI type.
    #[test]
    fn console_input_record_matches_the_windows_layout() {
        assert_eq!(std::mem::size_of::<ConsoleInputRecord>(), 20);
        assert_eq!(std::mem::align_of::<ConsoleInputRecord>(), 4);
        assert_eq!(std::mem::offset_of!(ConsoleInputRecord, key_event), 4);
        assert_eq!(
            std::mem::offset_of!(ConsoleKeyEventRecord, unicode_char),
            10
        );
    }

    fn key(key_down: bool, unicode_char: u16) -> ConsoleInputRecord {
        ConsoleInputRecord {
            event_type: 0x0001,
            key_event: ConsoleKeyEventRecord {
                key_down: i32::from(key_down),
                repeat_count: 1,
                unicode_char,
                ..ConsoleKeyEventRecord::default()
            },
        }
    }

    #[test]
    fn only_character_key_presses_wake_a_console_door_wait() {
        assert!(console_record_has_key_input(&key(true, 0x1b)));
        assert!(console_record_has_key_input(&key(true, u16::from(b'a'))));
        // Esc released, and an arrow or Shift press without VT input.
        assert!(!console_record_has_key_input(&key(false, 0x1b)));
        assert!(!console_record_has_key_input(&key(true, 0)));
        // Mouse, window resize, menu, and focus records.
        for event_type in [0x0002, 0x0004, 0x0008, 0x0010] {
            let record = ConsoleInputRecord {
                event_type,
                ..key(true, u16::from(b'a'))
            };
            assert!(!console_record_has_key_input(&record), "{event_type:#x}");
        }
    }
}
