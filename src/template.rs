use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/builtin_templates.rs"));

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
}
