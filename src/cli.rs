use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::providers::Provider;

#[derive(clap::ValueEnum, Clone, Debug, Default, PartialEq)]
pub enum OutputFormat {
    /// Wiki-style Markdown files (default)
    #[default]
    Markdown,
    /// Self-contained HTML site with sidebar navigation
    Html,
}

/// Metadata types that sfdoc can document.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MetadataType {
    Apex,
    Triggers,
    Flows,
    #[value(name = "validation-rules")]
    ValidationRules,
    Objects,
    Lwc,
    Flexipages,
    #[value(name = "custom-metadata")]
    CustomMetadata,
    Aura,
}

impl MetadataType {
    pub fn cli_name(self) -> &'static str {
        match self {
            Self::Apex => "apex",
            Self::Triggers => "triggers",
            Self::Flows => "flows",
            Self::ValidationRules => "validation-rules",
            Self::Objects => "objects",
            Self::Lwc => "lwc",
            Self::Flexipages => "flexipages",
            Self::CustomMetadata => "custom-metadata",
            Self::Aura => "aura",
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "sfdoc",
    about = "Generate wiki-style Markdown documentation for Salesforce source files",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate documentation from Salesforce source files
    Generate(GenerateArgs),
    /// Save an AI provider API key to the OS keychain
    Auth(AuthArgs),
    /// Show installation status and configuration
    Status,
}

#[derive(clap::Args, Debug)]
pub struct GenerateArgs {
    /// Path to Apex source directory
    #[arg(long, default_value = "force-app/main/default")]
    pub source_dir: PathBuf,

    /// Output directory for generated files.
    /// Defaults to `docs` for Markdown output and `site` for HTML output.
    #[arg(long, short)]
    pub output: Option<PathBuf>,

    /// AI provider to use for documentation generation
    #[arg(long, default_value = "gemini")]
    pub provider: Provider,

    /// Model to use (defaults to the provider's recommended model if not set)
    #[arg(long)]
    pub model: Option<String>,

    /// Maximum number of parallel API requests
    #[arg(long, default_value_t = 3, value_parser = parse_concurrency)]
    pub concurrency: usize,

    /// Maximum API requests per minute (0 = no limit).
    /// Use this to stay within your provider's RPM quota without lowering concurrency.
    #[arg(long, default_value_t = 0)]
    pub rpm: u32,

    /// Output format
    #[arg(long, default_value = "markdown")]
    pub format: OutputFormat,

    /// Regenerate all documentation, ignoring the incremental build cache
    #[arg(long)]
    pub force: bool,

    /// Only generate docs for these metadata types (comma-separated).
    /// Valid types: apex, triggers, flows, validation-rules, objects, lwc,
    /// flexipages, custom-metadata, aura.
    /// Default: all types.
    #[arg(long = "type", value_delimiter = ',')]
    pub types: Vec<MetadataType>,

    /// Only document files whose name matches this glob pattern (e.g. 'Order*', '*Service').
    /// Applied across all metadata types against the logical filename.
    #[arg(long)]
    pub name_filter: Option<String>,

    /// Only document items tagged with at least one of these labels (comma-separated).
    /// Tags are extracted from @tag annotations in ApexDoc comments.
    /// When --tag is specified, non-taggable metadata types (flows, objects, etc.) are excluded.
    #[arg(long = "tag", value_delimiter = ',')]
    pub tags: Vec<String>,

    /// Enable verbose logging
    #[arg(long, short)]
    pub verbose: bool,
}

impl GenerateArgs {
    /// Returns `true` if the given metadata type should be processed.
    /// When `--type` is omitted (empty vec), all types are selected.
    pub fn type_enabled(&self, t: MetadataType) -> bool {
        self.types.is_empty() || self.types.contains(&t)
    }

    /// Returns `true` if the given filename stem matches the `--name-filter` glob,
    /// or if no filter was specified.
    pub fn name_matches(&self, filename_stem: &str) -> bool {
        match &self.name_filter {
            None => true,
            Some(pattern) => {
                let glob = globset::Glob::new(pattern)
                    .unwrap_or_else(|_| globset::Glob::new("*").unwrap());
                glob.compile_matcher().is_match(filename_stem)
            }
        }
    }

    /// Returns `true` if the item's tags overlap with the `--tag` filter (OR logic, case-insensitive).
    /// Returns `true` when `--tag` is not specified.
    /// Returns `false` when `--tag` is specified but the item has no tags.
    pub fn tag_matches(&self, item_tags: &[String]) -> bool {
        if self.tags.is_empty() {
            return true;
        }
        item_tags.iter().any(|t| {
            self.tags
                .iter()
                .any(|f| f.eq_ignore_ascii_case(t))
        })
    }
}

fn parse_concurrency(s: &str) -> Result<usize, String> {
    let n: usize = s
        .parse()
        .map_err(|_| format!("'{s}' is not a valid integer"))?;
    if n == 0 {
        Err("--concurrency must be at least 1".to_string())
    } else {
        Ok(n)
    }
}

#[derive(clap::Args, Debug)]
pub struct AuthArgs {
    /// Provider to authenticate
    #[arg(long, default_value = "gemini")]
    pub provider: Provider,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse_generate(args: &[&str]) -> GenerateArgs {
        let mut full = vec!["sfdoc", "generate"];
        full.extend(args);
        let cli = Cli::try_parse_from(full).expect("CLI should parse");
        match cli.command {
            Commands::Generate(g) => g,
            _ => panic!("expected Generate command"),
        }
    }

    #[test]
    fn no_type_flag_enables_all() {
        let args = parse_generate(&[]);
        assert!(args.types.is_empty());
        assert!(args.type_enabled(MetadataType::Apex));
        assert!(args.type_enabled(MetadataType::Lwc));
        assert!(args.type_enabled(MetadataType::Aura));
    }

    #[test]
    fn single_type() {
        let args = parse_generate(&["--type", "apex"]);
        assert_eq!(args.types, vec![MetadataType::Apex]);
        assert!(args.type_enabled(MetadataType::Apex));
        assert!(!args.type_enabled(MetadataType::Flows));
    }

    #[test]
    fn comma_separated_types() {
        let args = parse_generate(&["--type", "apex,lwc,flows"]);
        assert_eq!(args.types.len(), 3);
        assert!(args.type_enabled(MetadataType::Apex));
        assert!(args.type_enabled(MetadataType::Lwc));
        assert!(args.type_enabled(MetadataType::Flows));
        assert!(!args.type_enabled(MetadataType::Triggers));
    }

    #[test]
    fn hyphenated_type_names() {
        let args = parse_generate(&["--type", "validation-rules,custom-metadata"]);
        assert!(args.type_enabled(MetadataType::ValidationRules));
        assert!(args.type_enabled(MetadataType::CustomMetadata));
        assert!(!args.type_enabled(MetadataType::Apex));
    }

    #[test]
    fn invalid_type_is_rejected() {
        let full = vec!["sfdoc", "generate", "--type", "invalid"];
        let result = Cli::try_parse_from(full);
        assert!(result.is_err());
    }

    #[test]
    fn repeated_type_flag() {
        let args = parse_generate(&["--type", "apex", "--type", "triggers"]);
        assert!(args.type_enabled(MetadataType::Apex));
        assert!(args.type_enabled(MetadataType::Triggers));
        assert!(!args.type_enabled(MetadataType::Flows));
    }

    #[test]
    fn name_filter_not_set_matches_all() {
        let args = parse_generate(&[]);
        assert!(args.name_matches("OrderService"));
        assert!(args.name_matches("anything"));
    }

    #[test]
    fn name_filter_matches_glob() {
        let args = parse_generate(&["--name-filter", "Order*"]);
        assert!(args.name_matches("OrderService"));
        assert!(args.name_matches("OrderHelper"));
        assert!(!args.name_matches("AccountService"));
    }

    #[test]
    fn name_filter_suffix_glob() {
        let args = parse_generate(&["--name-filter", "*Service"]);
        assert!(args.name_matches("OrderService"));
        assert!(!args.name_matches("OrderHelper"));
    }

    #[test]
    fn name_filter_contains_glob() {
        let args = parse_generate(&["--name-filter", "*Order*"]);
        assert!(args.name_matches("OrderService"));
        assert!(args.name_matches("MyOrderHelper"));
        assert!(!args.name_matches("AccountService"));
    }

    #[test]
    fn no_tag_flag_matches_all() {
        let args = parse_generate(&[]);
        assert!(args.tag_matches(&["billing".to_string()]));
        assert!(args.tag_matches(&[]));
    }

    #[test]
    fn tag_flag_matches_or_logic() {
        let args = parse_generate(&["--tag", "billing,integration"]);
        assert!(args.tag_matches(&["billing".to_string()]));
        assert!(args.tag_matches(&["integration".to_string()]));
        assert!(args.tag_matches(&["billing".to_string(), "other".to_string()]));
        assert!(!args.tag_matches(&["unrelated".to_string()]));
        assert!(!args.tag_matches(&[]));
    }

    #[test]
    fn tag_flag_case_insensitive() {
        let args = parse_generate(&["--tag", "Billing"]);
        assert!(args.tag_matches(&["billing".to_string()]));
        assert!(args.tag_matches(&["BILLING".to_string()]));
    }
}
