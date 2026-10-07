//! Structured failures from plugin lifecycle, settings, and host services.
use std::{
    backtrace::{Backtrace, BacktraceStatus},
    fmt,
    sync::Arc,
};

/// A recoverable plugin failure with diagnostic context and an optional cause.
#[derive(Clone, Debug)]
pub struct Error(Arc<Detail>);

#[derive(Debug)]
struct Detail {
    message: String,
    service: Option<&'static str>,
    status: Option<i32>,
    restart: bool,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
    backtrace: Backtrace,
}

impl Error {
    /// Describe a recoverable failure when no upstream cause is available.
    pub fn new(message: impl Into<String>) -> Self {
        Self(Arc::new(Detail {
            message: message.into(),
            service: None,
            status: None,
            restart: false,
            source: None,
            backtrace: Backtrace::capture(),
        }))
    }

    /// Preserve an upstream error as this failure's cause.
    pub fn from_cause(cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(Arc::new(Detail {
            message: "plugin operation failed".into(),
            service: None,
            status: None,
            restart: false,
            source: Some(Box::new(cause)),
            backtrace: Backtrace::capture(),
        }))
    }

    /// Record the numeric failure returned by a negotiated host service.
    pub fn host(service: &'static str, status: i32) -> Self {
        Self(Arc::new(Detail {
            message: format!("{service} service failed ({status})"),
            service: Some(service),
            status: Some(status),
            restart: false,
            source: None,
            backtrace: Backtrace::capture(),
        }))
    }

    /// Record an absent or incompatible optional host service.
    pub fn unavailable(service: &'static str) -> Self {
        Self(Arc::new(Detail {
            message: format!("{service} service unavailable; update host"),
            service: Some(service),
            status: None,
            restart: false,
            source: None,
            backtrace: Backtrace::capture(),
        }))
    }

    /// Report that a quarantined plugin requires a process restart.
    pub fn restart_required() -> Self {
        Self(Arc::new(Detail {
            message: "plugin panicked; restart Kenshi required".into(),
            service: None,
            status: None,
            restart: true,
            source: None,
            backtrace: Backtrace::capture(),
        }))
    }

    /// Return the host service associated with this failure, if any.
    pub fn service(&self) -> Option<&'static str> {
        self.0.service
    }
    /// Return the original host status code, if one was returned.
    pub fn status(&self) -> Option<i32> {
        self.0.status
    }
    /// Report whether restarting the host process is required.
    pub fn needs_restart(&self) -> bool {
        self.0.restart
    }
    /// Return the diagnostic backtrace captured when this failure was created.
    pub fn backtrace(&self) -> &Backtrace {
        &self.0.backtrace
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.message)?;
        if let Some(source) = &self.0.source {
            write!(f, ": {source}")?;
        }
        if self.0.backtrace.status() == BacktraceStatus::Captured {
            write!(f, "\n{}", self.0.backtrace)?;
        }
        Ok(())
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source.as_deref().map(|e| e as _)
    }
}
impl From<String> for Error {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}
impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}
impl From<std::io::Error> for Error {
    fn from(cause: std::io::Error) -> Self {
        Self::from_cause(cause)
    }
}
impl From<std::string::FromUtf8Error> for Error {
    fn from(cause: std::string::FromUtf8Error) -> Self {
        Self::from_cause(cause)
    }
}
impl From<toml::de::Error> for Error {
    fn from(cause: toml::de::Error) -> Self {
        Self::from_cause(cause)
    }
}
impl From<toml::ser::Error> for Error {
    fn from(cause: toml::ser::Error) -> Self {
        Self::from_cause(cause)
    }
}
impl From<Error> for String {
    fn from(error: Error) -> Self {
        error.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;
    #[test]
    fn io_cause_and_host_status_remain_inspectable() {
        let failure = Error::from(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
        assert_eq!(
            failure
                .source()
                .unwrap()
                .downcast_ref::<std::io::Error>()
                .unwrap()
                .kind(),
            std::io::ErrorKind::PermissionDenied
        );
        let failure = Error::host("graphics", 7);
        assert_eq!(failure.service(), Some("graphics"));
        assert_eq!(failure.status(), Some(7));
        assert!(Error::restart_required().needs_restart());
    }
}
