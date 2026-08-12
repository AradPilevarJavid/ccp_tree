use crate::cli::{Cli, Command};
use anyhow::{Context, Result};
use clap::parser::ValueSource;
use clap::ArgMatches;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE_NAME: &str = ".ccprc";

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub templates_dir: Option<PathBuf>,
    pub clipboard: Option<bool>,
    pub include_hidden: Option<bool>,
    pub no_ignore: Option<bool>,
    pub all: Option<bool>,
    pub exclude: Option<Vec<String>>,
    pub max_size: Option<u64>,
    pub max_chars: Option<u64>,
    pub head: Option<usize>,
    pub tail: Option<usize>,
    pub from_end: Option<bool>,
    pub tokens: Option<bool>,
    pub no_content: Option<bool>,
    pub structure: Option<bool>,
    pub reverse: Option<bool>,
    pub raw: Option<bool>,
    pub dry_run: Option<bool>,
    pub verbose: Option<bool>,
    pub quiet: Option<bool>,
    pub no_secret_scan: Option<bool>,
    pub force: Option<bool>,
}

pub fn load_default_config() -> Result<Config> {
    let global = global_config_path();
    let local = std::env::current_dir()
        .context("Failed to determine current directory")?
        .join(CONFIG_FILE_NAME);
    load_config_files(global.as_deref(), Some(&local))
}

pub fn load_config_files(global: Option<&Path>, local: Option<&Path>) -> Result<Config> {
    let mut config = Config::default();

    if let Some(path) = local.filter(|path| path.is_file()) {
        let same_file = global.is_some_and(|global| paths_refer_to_same_file(global, path));
        if !same_file {
            config.merge(read_config(path)?);
        }
    }
    if let Some(path) = global.filter(|path| path.is_file()) {
        config.merge(read_config(path)?);
    }

    config.validate()?;
    Ok(config)
}

pub fn apply_config(cli: &mut Cli, matches: &ArgMatches, config: &Config) -> Result<()> {
    #[cfg(not(feature = "clipboard"))]
    if config.clipboard == Some(true) {
        anyhow::bail!(
            "Config enables 'clipboard', but this ccp build does not include clipboard support"
        );
    }

    match &mut cli.command {
        None => apply_copy_config(cli, matches, config),
        Some(Command::Reverse(command)) => {
            let matches = matches
                .subcommand_matches("reverse")
                .expect("reverse matches should exist");
            set_if_not_cli(
                matches,
                "include_hidden",
                &mut command.include_hidden,
                config.include_hidden,
            );
            set_if_not_cli(
                matches,
                "no_ignore",
                &mut command.no_ignore,
                config.no_ignore,
            );
            set_if_not_cli(matches, "all", &mut command.all, config.all);
            set_if_not_cli(
                matches,
                "exclude",
                &mut command.exclude,
                config.exclude.clone(),
            );
            set_if_not_cli(matches, "max_size", &mut command.max_size, config.max_size);
            set_optional_if_not_cli(
                matches,
                "max_chars",
                &mut command.max_chars,
                config.max_chars,
            );
            let line_limit_on_cli =
                is_command_line(matches, "head") || is_command_line(matches, "tail");
            if !line_limit_on_cli {
                if let Some(head) = config.head {
                    command.head = Some(head);
                }
                if let Some(tail) = config.tail {
                    command.tail = Some(tail);
                }
            }
            set_if_not_cli(matches, "from_end", &mut command.from_end, config.from_end);
            set_if_not_cli(matches, "tokens", &mut command.tokens, config.tokens);
            set_if_not_cli(
                matches,
                "no_content",
                &mut command.no_content,
                config.no_content,
            );
            set_if_not_cli(matches, "dry_run", &mut command.dry_run, config.dry_run);
            set_if_not_cli(matches, "verbose", &mut command.verbose, config.verbose);
            set_if_not_cli(matches, "quiet", &mut command.quiet, config.quiet);
            set_if_not_cli(
                matches,
                "no_secret_scan",
                &mut command.no_secret_scan,
                config.no_secret_scan,
            );
            #[cfg(feature = "clipboard")]
            apply_clipboard_config(
                matches,
                &mut command.clipboard,
                command.no_clipboard,
                config.clipboard,
            );

            validate_content_options(
                command.head,
                command.tail,
                command.from_end,
                command.max_chars,
            )?;
            Ok(())
        }
        Some(Command::Generate(command)) | Some(Command::Create(command)) => {
            let name = if matches.subcommand_matches("generate").is_some() {
                "generate"
            } else {
                "create"
            };
            let matches = matches
                .subcommand_matches(name)
                .expect("generate/create matches should exist");
            set_if_not_cli(
                matches,
                "templates_dir",
                &mut command.templates_dir,
                config.templates_dir.clone(),
            );
            set_if_not_cli(matches, "force", &mut command.force, config.force);
            set_if_not_cli(matches, "dry_run", &mut command.dry_run, config.dry_run);
            set_if_not_cli(matches, "verbose", &mut command.verbose, config.verbose);
            set_if_not_cli(matches, "quiet", &mut command.quiet, config.quiet);
            Ok(())
        }
    }
}

fn apply_copy_config(cli: &mut Cli, matches: &ArgMatches, config: &Config) -> Result<()> {
    set_if_not_cli(
        matches,
        "include_hidden",
        &mut cli.include_hidden,
        config.include_hidden,
    );
    set_if_not_cli(matches, "no_ignore", &mut cli.no_ignore, config.no_ignore);
    set_if_not_cli(matches, "all", &mut cli.all, config.all);
    set_if_not_cli(matches, "exclude", &mut cli.exclude, config.exclude.clone());
    set_if_not_cli(matches, "max_size", &mut cli.max_size, config.max_size);
    set_optional_if_not_cli(matches, "max_chars", &mut cli.max_chars, config.max_chars);
    let line_limit_on_cli = is_command_line(matches, "head") || is_command_line(matches, "tail");
    if !line_limit_on_cli {
        if let Some(head) = config.head {
            cli.head = Some(head);
        }
        if let Some(tail) = config.tail {
            cli.tail = Some(tail);
        }
    }
    set_if_not_cli(matches, "from_end", &mut cli.from_end, config.from_end);
    set_if_not_cli(matches, "tokens", &mut cli.tokens, config.tokens);
    set_if_not_cli(
        matches,
        "no_content",
        &mut cli.no_content,
        config.no_content,
    );
    let output_mode_on_cli = ["structure", "reverse", "raw"]
        .iter()
        .any(|id| is_command_line(matches, id));
    if !output_mode_on_cli {
        if let Some(structure) = config.structure {
            cli.structure = structure;
        }
        if let Some(reverse) = config.reverse {
            cli.reverse = reverse;
        }
        if let Some(raw) = config.raw {
            cli.raw = raw;
        }
    }
    set_if_not_cli(matches, "dry_run", &mut cli.dry_run, config.dry_run);
    set_if_not_cli(matches, "verbose", &mut cli.verbose, config.verbose);
    set_if_not_cli(matches, "quiet", &mut cli.quiet, config.quiet);
    set_if_not_cli(
        matches,
        "no_secret_scan",
        &mut cli.no_secret_scan,
        config.no_secret_scan,
    );
    #[cfg(feature = "clipboard")]
    apply_clipboard_config(
        matches,
        &mut cli.clipboard,
        cli.no_clipboard,
        config.clipboard,
    );

    validate_content_options(cli.head, cli.tail, cli.from_end, cli.max_chars)?;
    if cli.raw && cli.structure {
        anyhow::bail!("Config enables both 'raw' and 'structure'; choose only one output mode");
    }
    Ok(())
}

#[cfg(feature = "clipboard")]
fn apply_clipboard_config(
    matches: &ArgMatches,
    clipboard: &mut bool,
    no_clipboard: bool,
    configured: Option<bool>,
) {
    if matches.value_source("no_clipboard") == Some(ValueSource::CommandLine) && no_clipboard {
        *clipboard = false;
    } else {
        set_if_not_cli(matches, "clipboard", clipboard, configured);
    }
}

fn set_if_not_cli<T: Clone>(matches: &ArgMatches, id: &str, target: &mut T, configured: Option<T>) {
    if matches.value_source(id) != Some(ValueSource::CommandLine) {
        if let Some(value) = configured {
            *target = value;
        }
    }
}

fn set_optional_if_not_cli<T>(
    matches: &ArgMatches,
    id: &str,
    target: &mut Option<T>,
    configured: Option<T>,
) {
    if !is_command_line(matches, id) {
        if let Some(value) = configured {
            *target = Some(value);
        }
    }
}

fn is_command_line(matches: &ArgMatches, id: &str) -> bool {
    matches.value_source(id) == Some(ValueSource::CommandLine)
}

fn validate_content_options(
    head: Option<usize>,
    tail: Option<usize>,
    from_end: bool,
    max_chars: Option<u64>,
) -> Result<()> {
    if head.is_some() && tail.is_some() {
        anyhow::bail!("Config enables both 'head' and 'tail'; choose only one");
    }
    if from_end && max_chars.is_none() {
        anyhow::bail!("Config enables 'from_end' without setting 'max_chars'");
    }
    Ok(())
}

fn read_config(path: &Path) -> Result<Config> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config {}", path.display()))?;
    let mut config: Config = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse config {}", path.display()))?;
    if let Some(templates_dir) = &config.templates_dir {
        if templates_dir.is_relative() {
            config.templates_dir = Some(
                path.parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(templates_dir),
            );
        }
    }
    Ok(config)
}

fn global_config_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .map(|home| home.join(CONFIG_FILE_NAME))
}

fn paths_refer_to_same_file(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

impl Config {
    fn merge(&mut self, higher_priority: Config) {
        if higher_priority.head.is_some() || higher_priority.tail.is_some() {
            self.head = None;
            self.tail = None;
        }
        if higher_priority.structure.is_some()
            || higher_priority.reverse.is_some()
            || higher_priority.raw.is_some()
        {
            self.structure = None;
            self.reverse = None;
            self.raw = None;
        }

        macro_rules! merge_fields {
            ($($field:ident),+ $(,)?) => {
                $(if higher_priority.$field.is_some() {
                    self.$field = higher_priority.$field;
                })+
            };
        }
        merge_fields!(
            templates_dir,
            clipboard,
            include_hidden,
            no_ignore,
            all,
            exclude,
            max_size,
            max_chars,
            head,
            tail,
            from_end,
            tokens,
            no_content,
            structure,
            reverse,
            raw,
            dry_run,
            verbose,
            quiet,
            no_secret_scan,
            force,
        );
    }

    fn validate(&self) -> Result<()> {
        validate_content_options(
            self.head,
            self.tail,
            self.from_end.unwrap_or(false),
            self.max_chars,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::{CommandFactory, FromArgMatches};

    fn parse(args: &[&str]) -> (Cli, ArgMatches) {
        let matches = Cli::command()
            .try_get_matches_from(args)
            .expect("arguments should parse");
        let cli = Cli::from_arg_matches(&matches).expect("CLI should be constructed");
        (cli, matches)
    }

    #[test]
    fn global_config_overrides_local_and_paths_are_relative_to_each_file() {
        let root = std::env::temp_dir().join(format!("ccp-config-test-{}", std::process::id()));
        let global_dir = root.join("home");
        let local_dir = root.join("project");
        let global_path = global_dir.join(CONFIG_FILE_NAME);
        let local_path = local_dir.join(CONFIG_FILE_NAME);

        fs::create_dir_all(&global_dir).expect("global directory should be created");
        fs::create_dir_all(&local_dir).expect("local directory should be created");
        fs::write(
            &global_path,
            "clipboard = true\nmax_size = 2000\ntemplates_dir = \"global-templates\"\n",
        )
        .expect("global config should be written");
        fs::write(
            &local_path,
            "clipboard = false\nmax_size = 3000\ntemplates_dir = \"local-templates\"\n",
        )
        .expect("local config should be written");

        let config =
            load_config_files(Some(&global_path), Some(&local_path)).expect("configs should load");

        assert_eq!(config.clipboard, Some(true));
        assert_eq!(config.max_size, Some(2000));
        assert_eq!(
            config.templates_dir,
            Some(global_dir.join("global-templates"))
        );

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn explicit_cli_values_override_config_values() {
        let (mut cli, matches) = parse(&["ccp", "--max-size", "1048576", "--raw"]);
        let config = Config {
            max_size: Some(42),
            raw: Some(false),
            structure: Some(true),
            ..Config::default()
        };

        apply_config(&mut cli, &matches, &config).expect("config should apply");

        assert_eq!(cli.max_size, 1_048_576);
        assert!(cli.raw);
        assert!(!cli.structure);
    }

    #[test]
    fn generate_uses_configured_templates_dir_unless_cli_overrides_it() {
        let (mut cli, matches) = parse(&["ccp", "create", "demo"]);
        let config = Config {
            templates_dir: Some(PathBuf::from("/configured/templates")),
            ..Config::default()
        };

        apply_config(&mut cli, &matches, &config).expect("config should apply");

        let Some(Command::Create(command)) = cli.command else {
            panic!("create command should parse");
        };
        assert_eq!(
            command.templates_dir,
            PathBuf::from("/configured/templates")
        );
    }

    #[cfg(feature = "clipboard")]
    #[test]
    fn no_clipboard_overrides_configured_clipboard() {
        let (mut cli, matches) = parse(&["ccp", "--no-clipboard"]);
        let config = Config {
            clipboard: Some(true),
            ..Config::default()
        };

        apply_config(&mut cli, &matches, &config).expect("config should apply");

        assert!(!cli.clipboard);
    }

    #[test]
    fn explicit_tail_replaces_configured_head() {
        let (mut cli, matches) = parse(&["ccp", "--tail", "5"]);
        let config = Config {
            head: Some(10),
            ..Config::default()
        };

        apply_config(&mut cli, &matches, &config).expect("config should apply");

        assert_eq!(cli.head, None);
        assert_eq!(cli.tail, Some(5));
    }

    #[test]
    fn explicit_from_end_can_use_configured_max_chars() {
        let (mut cli, matches) = parse(&["ccp", "-r"]);
        let config = Config {
            max_chars: Some(80),
            ..Config::default()
        };

        apply_config(&mut cli, &matches, &config).expect("config should apply");

        assert!(cli.from_end);
        assert_eq!(cli.max_chars, Some(80));
    }

    #[test]
    fn higher_priority_line_limit_replaces_lower_priority_limit() {
        let mut lower = Config {
            tail: Some(20),
            ..Config::default()
        };
        lower.merge(Config {
            head: Some(10),
            ..Config::default()
        });

        assert_eq!(lower.head, Some(10));
        assert_eq!(lower.tail, None);
    }

    #[test]
    fn unknown_config_fields_report_the_file_path() {
        let root =
            std::env::temp_dir().join(format!("ccp-config-error-test-{}", std::process::id()));
        let path = root.join(CONFIG_FILE_NAME);

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&path, "not_a_real_setting = true\n").expect("config should be written");

        let error =
            load_config_files(None, Some(&path)).expect_err("unknown field should be rejected");

        assert!(error.to_string().contains(&path.display().to_string()));

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn invalid_config_combinations_are_rejected() {
        let config = Config {
            head: Some(10),
            tail: Some(10),
            ..Config::default()
        };

        let error = config.validate().expect_err("config should be invalid");
        assert!(error.to_string().contains("both 'head' and 'tail'"));
    }
}
