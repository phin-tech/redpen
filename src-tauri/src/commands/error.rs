use redpen_core::sidecar::SidecarError;
use std::fmt;

/// Typed error enum for Tauri command handlers.
///
/// Internal code should use these variants instead of ad-hoc strings.
/// Tauri commands return `Result<T, CommandError>`, and the `Into<tauri::InvokeError>`
/// implementation (via `Display` + `Into<String>`) bridges to the frontend.
#[derive(Debug)]
pub enum CommandError {
    /// File system I/O failure (read, write, delete, create_dir).
    Io(std::io::Error),
    /// Git operation failure (discover, status, diff, ref lookup).
    Git(git2::Error),
    /// JSON serialization / deserialization failure.
    Json(serde_json::Error),
    /// Sidecar file load/save failure.
    Sidecar(SidecarError),
    /// A requested resource was not found (annotation, file at ref, etc.).
    NotFound(String),
    /// An invalid argument was provided by the caller.
    InvalidArgument(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::Io(e) => write!(f, "{}", e),
            CommandError::Git(e) => write!(f, "{}", e),
            CommandError::Json(e) => write!(f, "{}", e),
            CommandError::Sidecar(e) => write!(f, "{}", e),
            CommandError::NotFound(msg) => write!(f, "{}", msg),
            CommandError::InvalidArgument(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        CommandError::Io(e)
    }
}

impl From<git2::Error> for CommandError {
    fn from(e: git2::Error) -> Self {
        CommandError::Git(e)
    }
}

impl From<serde_json::Error> for CommandError {
    fn from(e: serde_json::Error) -> Self {
        CommandError::Json(e)
    }
}

impl From<SidecarError> for CommandError {
    fn from(e: SidecarError) -> Self {
        CommandError::Sidecar(e)
    }
}

// Tauri requires the error type to implement `Into<tauri::InvokeError>`.
// `InvokeError` implements `From<String>`, so providing `Into<String>` suffices.
impl From<CommandError> for String {
    fn from(e: CommandError) -> Self {
        e.to_string()
    }
}

/// Shorthand for command return types.
pub type CommandResult<T> = Result<T, CommandError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_displays_message() {
        let err = CommandError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert_eq!(err.to_string(), "file not found");
    }

    #[test]
    fn not_found_displays_message() {
        let err = CommandError::NotFound("Annotation not found".into());
        assert_eq!(err.to_string(), "Annotation not found");
    }

    #[test]
    fn invalid_argument_displays_message() {
        let err = CommandError::InvalidArgument("bad input".into());
        assert_eq!(err.to_string(), "bad input");
    }

    #[test]
    fn into_string_works() {
        let err = CommandError::NotFound("test".into());
        let s: String = err.into();
        assert_eq!(s, "test");
    }
}
