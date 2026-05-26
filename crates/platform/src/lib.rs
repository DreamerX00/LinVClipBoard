mod error;
pub mod ipc;
mod traits;

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

pub use error::PlatformError;
pub use traits::ClipboardMonitor;
pub use traits::ClipboardProvider;
pub use traits::InputSimulator;
pub use traits::IpcListener;
pub use traits::IpcStream;
pub use traits::IpcTransport;
pub use traits::ServiceManager;

#[cfg(unix)]
pub use unix::{
    UnixClipboardMonitor, UnixClipboardProvider, UnixInputSimulator, UnixIpcTransport,
    UnixServiceManager,
};

#[cfg(windows)]
pub use windows::{
    WindowsClipboardMonitor, WindowsClipboardProvider, WindowsInputSimulator, WindowsIpcTransport,
    WindowsServiceManager,
};
