use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/builtin_templates.rs"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableTemplate {
    pub name: String,
    pub source: TemplateSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSource {
    Builtin,
    Custom(PathBuf),
}

pub fn list_templates(templates_dir: &Path) -> Result<Vec<AvailableTemplate>> {
    let mut templates = BUILTIN_TEMPLATES
        .iter()
        .map(|(name, _)| AvailableTemplate {
            name: (*name).to_string(),
            source: TemplateSource::Builtin,
        })
        .collect::<Vec<_>>();

    if templates_dir.exists() {
        if !templates_dir.is_dir() {
            bail!("Templates path {} is not a directory", templates_dir.display());
        }

        for entry in fs::read_dir(templates_dir)
            .with_context(|| format!("Failed to read templates directory {}", templates_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let is_tree_file = path.extension().and_then(|extension| extension.to_str()) == Some("tree");
            let is_extensionless = path.extension().is_none();
            if !is_tree_file && !is_extensionless {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };

            if let Some(template) = templates.iter_mut().find(|template| template.name == name) {
                template.source = TemplateSource::Custom(path);
            } else {
                templates.push(AvailableTemplate {
                    name: name.to_string(),
                    source: TemplateSource::Custom(path),
                });
            }
        }
    }

    templates.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(templates)
}

pub fn load_template(templates_dir: &Path, name: &str) -> Result<String> {
    let candidates = [
        templates_dir.join(name),
        templates_dir.join(format!("{name}.tree")),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return fs::read_to_string(&candidate)
                .with_context(|| format!("Failed to read template {}", candidate.display()));
        }
    }

    let builtin_name = name.strip_suffix(".tree").unwrap_or(name);
    if let Some((_, template)) = BUILTIN_TEMPLATES
        .iter()
        .find(|(template_name, _)| *template_name == builtin_name)
    {
        return Ok((*template).to_string());
    }

    let builtin_templates = if BUILTIN_TEMPLATES.is_empty() {
        "none".to_string()
    } else {
        BUILTIN_TEMPLATES
            .iter()
            .map(|(template_name, _)| *template_name)
            .collect::<Vec<_>>()
            .join(", ")
    };

    bail!(
        "Template '{name}' was not found in {} or built-in templates ({builtin_templates})",
        templates_dir.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_tree_definition;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn load_template_falls_back_to_builtin_templates() {
        let template = load_template(Path::new("missing-template-dir"), "python")
            .expect("python should be available as a built-in template");

        assert!(template.contains("main.py"));
        assert!(template.contains("Hello from ccp"));
    }

    #[test]
    fn expected_builtin_templates_are_available_and_valid() {
        let expected = [
            "c",
            "cpp",
            "datascience",
            "go",
            "java",
            "node",
            "python",
            "react",
            "ruby",
            "rust",
            "typescript",
            "web",
        ];
        let actual = BUILTIN_TEMPLATES
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>();

        assert_eq!(actual, expected);

        for (name, template) in BUILTIN_TEMPLATES {
            let nodes = parse_tree_definition(template)
                .unwrap_or_else(|error| panic!("built-in template '{name}' is invalid: {error}"));
            assert!(
                !nodes.is_empty(),
                "built-in template '{name}' should not be empty"
            );
        }
    }

    #[test]
    fn list_templates_includes_builtins_and_custom_templates() {
        let temp_dir = std::env::temp_dir().join(format!(
            "ccp-template-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        fs::create_dir(&temp_dir).expect("temporary directory");
        fs::write(temp_dir.join("python.tree"), "custom").expect("custom template");
        fs::write(temp_dir.join("custom.tree"), "custom").expect("custom template");
        fs::write(temp_dir.join("ignored.txt"), "ignored").expect("ignored file");

        let templates = list_templates(&temp_dir).expect("templates should list");
        assert!(templates.iter().any(|template| {
            template.name == "python"
                && template.source == TemplateSource::Custom(temp_dir.join("python.tree"))
        }));
        assert!(templates.iter().any(|template| template.name == "custom"));
        assert!(!templates.iter().any(|template| template.name == "ignored"));
        fs::remove_dir_all(temp_dir).expect("remove temporary directory");
    }
}
