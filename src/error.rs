use thiserror::Error;

#[derive(Debug, Error)]
pub enum SfdocError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Gemini API error: {0}")]
    GeminiApi(String),

    #[error("Failed to parse Gemini response: {0}")]
    ParseError(String),

    #[error("No Apex files found in {0}")]
    NoFilesFound(String),
}
