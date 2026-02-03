use crate::error::AppError;

const ALLOWED_COMMANDS: &[&str] = &[
    "rg", "grep", "head", "tail", "cat", "wc", "find", "ls", "sort", "uniq", "sed", "awk", "tr",
    "cut", "diff", "file", "stat", "du", "tree",
];

pub fn is_allowed(command: &str) -> bool {
    ALLOWED_COMMANDS.contains(&command)
}

/// Reject arguments that could enable shell injection or path traversal.
pub fn validate_args(args: &[String]) -> Result<(), AppError> {
    for arg in args {
        // Reject path traversal
        if arg.contains("..") {
            return Err(AppError::Forbidden(
                "Path traversal in arguments not allowed".into(),
            ));
        }

        // Reject shell metacharacters
        for ch in &['|', ';', '`', '$', '&', '\n', '\r'] {
            if arg.contains(*ch) {
                return Err(AppError::Forbidden(format!(
                    "Shell metacharacter '{}' not allowed in arguments",
                    ch
                )));
            }
        }

        // Reject $() pattern
        if arg.contains("$(") {
            return Err(AppError::Forbidden(
                "Command substitution not allowed in arguments".into(),
            ));
        }
    }
    Ok(())
}
