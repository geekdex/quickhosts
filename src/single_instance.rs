use std::fmt;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SingleInstanceMessage {
    Focus,
}

#[derive(Debug)]
pub enum SingleInstanceError {
    AlreadyRunning,
    Io(io::Error),
}

impl fmt::Display for SingleInstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "Another instance of QuickHosts is already running"),
            Self::Io(e) => write!(f, "I/O error in single-instance management: {}", e),
        }
    }
}

impl std::error::Error for SingleInstanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AlreadyRunning => None,
            Self::Io(e) => Some(e),
        }
    }
}

pub struct SingleInstanceGuard {
    #[cfg(unix)]
    socket_path: Option<PathBuf>,
    #[cfg(windows)]
    lock_path: Option<PathBuf>,
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(path) = self.socket_path.take() {
            let _ = std::fs::remove_file(path);
        }
        #[cfg(windows)]
        if let Some(path) = self.lock_path.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(unix)]
fn resolve_socket_path() -> PathBuf {
    if let Some(runtime_dir) = dirs::runtime_dir() {
        runtime_dir.join("quickhosts.sock")
    } else {
        crate::storage::get_app_dir().join("quickhosts.sock")
    }
}

#[cfg(unix)]
pub fn acquire(
) -> Result<(SingleInstanceGuard, Receiver<SingleInstanceMessage>), SingleInstanceError> {
    acquire_internal(resolve_socket_path())
}

#[cfg(unix)]
pub fn acquire_internal(
    sock_path: PathBuf,
) -> Result<(SingleInstanceGuard, Receiver<SingleInstanceMessage>), SingleInstanceError> {
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::time::Duration;

    if sock_path.exists() {
        if let Ok(mut stream) = UnixStream::connect(&sock_path) {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(300)));
            let _ = stream.write_all(b"FOCUS\n");
            let _ = stream.flush();
            let mut reply = [0u8; 4];
            let _ = stream.read(&mut reply);
            return Err(SingleInstanceError::AlreadyRunning);
        }
        // Stale socket from dead or killed process
        let _ = std::fs::remove_file(&sock_path);
    }

    let listener = match UnixListener::bind(&sock_path) {
        Ok(l) => l,
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            std::thread::sleep(Duration::from_millis(50));
            if let Ok(mut stream) = UnixStream::connect(&sock_path) {
                let _ = stream.write_all(b"FOCUS\n");
                let _ = stream.flush();
                return Err(SingleInstanceError::AlreadyRunning);
            }
            let _ = std::fs::remove_file(&sock_path);
            UnixListener::bind(&sock_path).map_err(SingleInstanceError::Io)?
        }
        Err(e) => return Err(SingleInstanceError::Io(e)),
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let sock_path_for_guard = sock_path.clone();

    std::thread::Builder::new()
        .name("single-instance-ipc".to_string())
        .spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(mut s) => {
                        let mut buf = [0u8; 32];
                        if let Ok(n) = s.read(&mut buf) {
                            if let Ok(cmd) = std::str::from_utf8(&buf[..n]) {
                                if cmd.trim() == "FOCUS" {
                                    let _ = tx.send(SingleInstanceMessage::Focus);
                                    let _ = s.write_all(b"OK\n");
                                    let _ = s.flush();
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        })
        .map_err(SingleInstanceError::Io)?;

    Ok((
        SingleInstanceGuard {
            socket_path: Some(sock_path_for_guard),
        },
        rx,
    ))
}

#[cfg(windows)]
fn resolve_lock_path() -> PathBuf {
    crate::storage::get_app_dir().join("quickhosts.instance")
}

#[cfg(windows)]
pub fn acquire(
) -> Result<(SingleInstanceGuard, Receiver<SingleInstanceMessage>), SingleInstanceError> {
    acquire_internal(resolve_lock_path())
}

#[cfg(windows)]
pub fn acquire_internal(
    lock_path: PathBuf,
) -> Result<(SingleInstanceGuard, Receiver<SingleInstanceMessage>), SingleInstanceError> {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;

    if lock_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            let mut lines = content.lines();
            if let (Some(port_str), Some(token)) = (lines.next(), lines.next()) {
                if let Ok(port) = port_str.trim().parse::<u16>() {
                    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
                    if let Ok(mut stream) =
                        TcpStream::connect_timeout(&addr, Duration::from_millis(300))
                    {
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
                        let _ = stream.set_write_timeout(Some(Duration::from_millis(300)));
                        let msg = format!("{} FOCUS\n", token.trim());
                        if stream.write_all(msg.as_bytes()).is_ok() && stream.flush().is_ok() {
                            let mut reply = [0u8; 4];
                            let _ = stream.read(&mut reply);
                            return Err(SingleInstanceError::AlreadyRunning);
                        }
                    }
                }
            }
        }
        let _ = std::fs::remove_file(&lock_path);
    }

    let listener = TcpListener::bind("127.0.0.1:0").map_err(SingleInstanceError::Io)?;
    let port = listener
        .local_addr()
        .map_err(SingleInstanceError::Io)?
        .port();
    let secret_token = uuid::Uuid::new_v4().to_string();

    let lock_content = format!("{}\n{}\n", port, secret_token);
    std::fs::write(&lock_path, lock_content).map_err(SingleInstanceError::Io)?;

    let (tx, rx) = std::sync::mpsc::channel();
    let lock_path_for_guard = lock_path.clone();
    let token_clone = secret_token.clone();

    std::thread::Builder::new()
        .name("single-instance-ipc".to_string())
        .spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(mut s) => {
                        let mut buf = [0u8; 128];
                        if let Ok(n) = s.read(&mut buf) {
                            if let Ok(cmd) = std::str::from_utf8(&buf[..n]) {
                                let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
                                if parts.len() == 2
                                    && parts[0] == token_clone
                                    && parts[1] == "FOCUS"
                                {
                                    let _ = tx.send(SingleInstanceMessage::Focus);
                                    let _ = s.write_all(b"OK\n");
                                    let _ = s.flush();
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        })
        .map_err(SingleInstanceError::Io)?;

    Ok((
        SingleInstanceGuard {
            lock_path: Some(lock_path_for_guard),
        },
        rx,
    ))
}

#[cfg(not(any(unix, windows)))]
pub fn acquire(
) -> Result<(SingleInstanceGuard, Receiver<SingleInstanceMessage>), SingleInstanceError> {
    let (_tx, rx) = std::sync::mpsc::channel();
    Ok((SingleInstanceGuard {}, rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_instance_acquisition_and_focus() {
        let temp_dir = std::env::temp_dir().join(format!("qh_test_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let test_path = temp_dir.join("test.sock");

        // 1. First acquire succeeds
        let (guard, rx) =
            acquire_internal(test_path.clone()).expect("First acquire should succeed");

        // 2. Second acquire returns AlreadyRunning
        let second_res = acquire_internal(test_path.clone());
        assert!(matches!(
            second_res,
            Err(SingleInstanceError::AlreadyRunning)
        ));

        // 3. First instance should receive Focus message
        let msg = rx.recv_timeout(std::time::Duration::from_secs(1));
        assert_eq!(msg.unwrap(), SingleInstanceMessage::Focus);

        // 4. Drop first guard
        drop(guard);

        // 5. Subsequent acquire should succeed again
        let third_res = acquire_internal(test_path.clone());
        assert!(third_res.is_ok());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
