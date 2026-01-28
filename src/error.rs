use thiserror::Error;

#[derive(Error, Debug)]
pub enum AetherError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Jujutsu command failed: {message} (exit code: {exit_code})")]
    Jj { message: String, exit_code: i32 },

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("Port allocation failed: {0}")]
    PortAllocation(String),

    #[error("Context injection failed: {0}")]
    ContextInjection(String),

    #[error("State management error: {0}")]
    State(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, AetherError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AetherError::Config("test error".to_string());
        assert_eq!(err.to_string(), "Configuration error: test error");
    }

    #[test]
    fn test_jj_error() {
        let err = AetherError::Jj {
            message: "command failed".to_string(),
            exit_code: 1,
        };
        assert!(err.to_string().contains("exit code: 1"));
    }
}
