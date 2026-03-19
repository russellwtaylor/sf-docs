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
    /// Returns `None` for Gemini (which uses its own client).
    pub fn base_url(&self) -> Option<&'static str> {
        match self {
            Provider::Gemini => None,
            Provider::Groq => Some("https://api.groq.com/openai/v1"),
            Provider::OpenAi => Some("https://api.openai.com/v1"),
            Provider::Ollama => Some("http://localhost:11434/v1"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_providers_have_default_models() {
        for provider in Provider::all() {
            assert!(!provider.default_model().is_empty(), "{:?} missing default model", provider);
        }
    }

    #[test]
    fn all_providers_have_display_names() {
        for provider in Provider::all() {
            assert!(!provider.display_name().is_empty());
        }
    }

    #[test]
    fn all_providers_have_keychain_keys() {
        for provider in Provider::all() {
            assert!(!provider.keychain_key().is_empty());
        }
    }

    #[test]
    fn all_providers_have_cli_names() {
        for provider in Provider::all() {
            assert!(!provider.cli_name().is_empty());
        }
    }

    #[test]
    fn gemini_has_no_base_url() {
        assert!(Provider::Gemini.base_url().is_none());
    }

    #[test]
    fn openai_compat_providers_have_base_urls() {
        assert!(Provider::Groq.base_url().is_some());
        assert!(Provider::OpenAi.base_url().is_some());
        assert!(Provider::Ollama.base_url().is_some());
    }

    #[test]
    fn ollama_does_not_require_api_key() {
        assert!(!Provider::Ollama.requires_api_key());
    }

    #[test]
    fn non_ollama_providers_require_api_key() {
        assert!(Provider::Gemini.requires_api_key());
        assert!(Provider::Groq.requires_api_key());
        assert!(Provider::OpenAi.requires_api_key());
    }

    #[test]
    fn ollama_has_no_env_var() {
        assert!(Provider::Ollama.env_var().is_none());
    }

    #[test]
    fn non_ollama_providers_have_env_vars() {
        assert!(Provider::Gemini.env_var().is_some());
        assert!(Provider::Groq.env_var().is_some());
        assert!(Provider::OpenAi.env_var().is_some());
    }

    #[test]
    fn display_trait_uses_display_name() {
        assert_eq!(format!("{}", Provider::Gemini), "Google Gemini");
        assert_eq!(format!("{}", Provider::Ollama), "Ollama (local)");
    }

    #[test]
    fn all_returns_four_providers() {
        assert_eq!(Provider::all().len(), 4);
    }

    #[test]
    fn provider_equality() {
        assert_eq!(Provider::Gemini, Provider::Gemini);
        assert_ne!(Provider::Gemini, Provider::Groq);
    }
}
