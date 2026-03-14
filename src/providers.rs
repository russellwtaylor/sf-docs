use std::fmt;

/// Supported AI providers.
#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
pub enum Provider {
    /// Google Gemini (default — free tier available)
    Gemini,
    /// Groq cloud inference (free tier, very fast)
    Groq,
    /// OpenAI (paid)
    #[value(name = "openai")]
    OpenAi,
    /// Ollama — run models locally for free
    Ollama,
}

impl Provider {
    /// Default model to use when `--model` is not specified.
    pub fn default_model(&self) -> &'static str {
        match self {
            Provider::Gemini => "gemini-2.5-flash",
            Provider::Groq => "llama-3.3-70b-versatile",
            Provider::OpenAi => "gpt-4o-mini",
            Provider::Ollama => "llama3.2",
        }
    }

    /// Human-readable name shown in status output.
    pub fn display_name(&self) -> &'static str {
        match self {
            Provider::Gemini => "Google Gemini",
            Provider::Groq => "Groq",
            Provider::OpenAi => "OpenAI",
            Provider::Ollama => "Ollama (local)",
        }
    }

    /// Keychain entry name used to store this provider's API key.
    pub fn keychain_key(&self) -> &'static str {
        match self {
            Provider::Gemini => "gemini_api_key",
            Provider::Groq => "groq_api_key",
            Provider::OpenAi => "openai_api_key",
            Provider::Ollama => "ollama_api_key",
        }
    }

    /// Environment variable that overrides the keychain for this provider.
    pub fn env_var(&self) -> Option<&'static str> {
        match self {
            Provider::Gemini => Some("GEMINI_API_KEY"),
            Provider::Groq => Some("GROQ_API_KEY"),
            Provider::OpenAi => Some("OPENAI_API_KEY"),
            Provider::Ollama => None,
        }
    }

    /// Whether this provider needs an API key at all.
    pub fn requires_api_key(&self) -> bool {
        !matches!(self, Provider::Ollama)
    }

    /// Base URL for the OpenAI-compatible chat completions API.
    /// Only used for non-Gemini providers.
    pub fn base_url(&self) -> &'static str {
        match self {
            Provider::Gemini => unreachable!("Gemini uses its own client"),
            Provider::Groq => "https://api.groq.com/openai/v1",
            Provider::OpenAi => "https://api.openai.com/v1",
            Provider::Ollama => "http://localhost:11434/v1",
        }
    }

    /// All providers, in display order.
    pub fn all() -> &'static [Provider] {
        &[
            Provider::Gemini,
            Provider::Groq,
            Provider::OpenAi,
            Provider::Ollama,
        ]
    }

    /// The CLI value name (used in error messages and status output).
    pub fn cli_name(&self) -> &'static str {
        match self {
            Provider::Gemini => "gemini",
            Provider::Groq => "groq",
            Provider::OpenAi => "openai",
            Provider::Ollama => "ollama",
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
