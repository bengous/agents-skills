use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

const NAME_MAX_CHARS: usize = 64;
const DESCRIPTION_MAX_CHARS: usize = 1024;
const COMPATIBILITY_MAX_CHARS: usize = 500;
const SPEC_URL: &str = "https://agentskills.io/specification";

pub fn run_cli(args: impl IntoIterator<Item = String>) -> Result<String, String> {
    let args = args.into_iter().collect::<Vec<_>>();

    match args.as_slice() {
        [validate, frontmatter, rest @ ..]
            if validate == "validate" && frontmatter == "frontmatter" =>
        {
            validate_frontmatter(rest)
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    [
        "Usage:",
        "  skills-tools validate frontmatter [paths...]",
        "",
        "Examples:",
        "  skills-tools validate frontmatter",
        "  skills-tools validate frontmatter content-architect/SKILL.md",
    ]
    .join("\n")
}

fn validate_frontmatter(raw_paths: &[String]) -> Result<String, String> {
    let mut paths = if raw_paths.is_empty() {
        discover_skill_files(".")
            .map_err(|error| format!("Failed to discover SKILL.md files: {error}"))?
    } else {
        raw_paths
            .iter()
            .map(PathBuf::from)
            .filter(|path| path.file_name().is_some_and(|name| name == "SKILL.md"))
            .collect::<Vec<_>>()
    };

    paths.sort();
    paths.dedup();
    let validated_count = paths.len();

    if paths.is_empty() {
        return Ok(String::new());
    }

    let mut problems = Vec::new();

    for path in paths {
        match validate_skill_file(&path) {
            Ok(()) => {}
            Err(file_problems) => problems.extend(file_problems),
        }
    }

    if problems.is_empty() {
        return Ok(format!("Validated {validated_count} skill file(s)."));
    }

    let mut message = String::from("Skill frontmatter validation failed:\n");
    for problem in problems {
        message.push_str("- ");
        message.push_str(&problem);
        message.push('\n');
    }
    let _ = writeln!(message, "Spec: {SPEC_URL}");

    Err(message.trim_end().to_owned())
}

fn discover_skill_files(root: impl AsRef<Path>) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut found = Vec::new();
    discover_skill_files_recursive(root.as_ref(), &mut found)?;
    Ok(found)
}

fn discover_skill_files_recursive(
    dir: &Path,
    found: &mut Vec<PathBuf>,
) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        if entry.file_type()?.is_dir() {
            if should_skip_dir(&file_name) {
                continue;
            }
            discover_skill_files_recursive(&path, found)?;
            continue;
        }

        if file_name == "SKILL.md" {
            found.push(path);
        }
    }

    Ok(())
}

fn should_skip_dir(name: &str) -> bool {
    matches!(name, ".git" | "target" | "node_modules")
}

fn validate_skill_file(path: &Path) -> Result<(), Vec<String>> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            return Err(vec![format!(
                "{}: failed to read file: {error}",
                path.display()
            )]);
        }
    };

    let frontmatter = match extract_frontmatter(&content) {
        Ok(frontmatter) => frontmatter,
        Err(error) => {
            return Err(vec![format!("{}: {error}", path.display())]);
        }
    };

    let parsed = match parse_frontmatter(frontmatter) {
        Ok(parsed) => parsed,
        Err(error) => {
            return Err(vec![format!(
                "{}: invalid YAML frontmatter: {error}",
                path.display()
            )]);
        }
    };

    let mut problems = Vec::new();
    validate_name(path, &parsed, &mut problems);
    validate_description(path, &parsed, &mut problems);
    validate_compatibility(path, &parsed, &mut problems);

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

fn extract_frontmatter(content: &str) -> Result<&str, String> {
    let mut lines = content.lines();
    match lines.next() {
        Some("---") => {}
        _ => return Err("missing opening frontmatter delimiter (`---`)".to_owned()),
    }

    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .ok_or_else(|| "missing opening frontmatter delimiter (`---`)".to_owned())?;

    let end_index = rest
        .find("\n---\n")
        .or_else(|| rest.find("\n---\r\n"))
        .or_else(|| rest.find("\r\n---\n"))
        .or_else(|| rest.find("\r\n---\r\n"));

    let Some(end_index) = end_index else {
        return Err("missing closing frontmatter delimiter (`---`)".to_owned());
    };

    Ok(&rest[..end_index])
}

fn parse_frontmatter(
    frontmatter: &str,
) -> Result<BTreeMap<String, serde_yaml::Value>, serde_yaml::Error> {
    serde_yaml::from_str(frontmatter)
}

fn validate_name(
    path: &Path,
    parsed: &BTreeMap<String, serde_yaml::Value>,
    problems: &mut Vec<String>,
) {
    let Some(name) = get_string_field(parsed, "name") else {
        problems.push(format!(
            "{}: `name` must be a non-empty string",
            path.display()
        ));
        return;
    };

    let length = name.chars().count();
    if length == 0 || length > NAME_MAX_CHARS {
        problems.push(format!(
            "{}: `name` length is {length}, expected 1-{NAME_MAX_CHARS}",
            path.display()
        ));
    }

    if !is_valid_skill_name(name) {
        problems.push(format!(
            "{}: `name` must use lowercase letters, numbers, and single hyphens only, without leading/trailing hyphens",
            path.display()
        ));
    }

    let Some(parent_name) = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
    else {
        problems.push(format!(
            "{}: could not determine parent directory name",
            path.display()
        ));
        return;
    };

    if name != parent_name {
        problems.push(format!(
            "{}: `name` is `{name}` but parent directory is `{parent_name}`",
            path.display()
        ));
    }
}

fn validate_description(
    path: &Path,
    parsed: &BTreeMap<String, serde_yaml::Value>,
    problems: &mut Vec<String>,
) {
    let Some(description) = get_string_field(parsed, "description") else {
        problems.push(format!(
            "{}: `description` must be a non-empty string",
            path.display()
        ));
        return;
    };

    let length = description.chars().count();
    if length == 0 || length > DESCRIPTION_MAX_CHARS {
        problems.push(format!(
            "{}: `description` length is {length}, expected 1-{DESCRIPTION_MAX_CHARS}",
            path.display()
        ));
    }
}

fn validate_compatibility(
    path: &Path,
    parsed: &BTreeMap<String, serde_yaml::Value>,
    problems: &mut Vec<String>,
) {
    let Some(value) = parsed.get("compatibility") else {
        return;
    };

    let compatibility = yaml_value_to_single_line(value);
    let length = compatibility.chars().count();
    if length == 0 || length > COMPATIBILITY_MAX_CHARS {
        problems.push(format!(
            "{}: `compatibility` length is {length}, expected 1-{COMPATIBILITY_MAX_CHARS}",
            path.display()
        ));
    }
}

fn get_string_field<'a>(
    parsed: &'a BTreeMap<String, serde_yaml::Value>,
    field: &str,
) -> Option<&'a str> {
    parsed
        .get(field)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn yaml_value_to_single_line(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(text) => text.trim().to_owned(),
        other => serde_yaml::to_string(other)
            .unwrap_or_default()
            .replace('\n', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn is_valid_skill_name(name: &str) -> bool {
    if name.starts_with('-') || name.ends_with('-') || name.contains("--") {
        return false;
    }

    name.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    })
}

#[cfg(test)]
mod tests {
    use super::{COMPATIBILITY_MAX_CHARS, extract_frontmatter, validate_skill_file};
    use std::error::Error;
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn rejects_description_longer_than_spec_limit() -> Result<(), Box<dyn Error>> {
        let path = write_skill_file(
            "bad-skill",
            &format!(
                "---\nname: bad-skill\ndescription: \"{}\"\n---\n",
                "x".repeat(1025)
            ),
        )?;

        let problems = match validate_skill_file(&path) {
            Ok(()) => return Err(Box::new(io::Error::other("description > 1024 must fail"))),
            Err(problems) => problems,
        };

        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("`description` length is 1025, expected 1-1024")),
            "unexpected problems: {problems:#?}"
        );

        Ok(())
    }

    #[test]
    fn accepts_multiline_pipe_description() -> Result<(), Box<dyn Error>> {
        let path = write_skill_file(
            "good-skill",
            "---\nname: good-skill\ndescription: |\n  line one\n  line two\n---\n",
        )?;

        if let Err(problems) = validate_skill_file(&path) {
            return Err(Box::new(io::Error::other(problems.join("; "))));
        }

        Ok(())
    }

    #[test]
    fn rejects_name_that_does_not_match_parent_directory() -> Result<(), Box<dyn Error>> {
        let path = write_skill_file(
            "folder-name",
            "---\nname: other-name\ndescription: valid description\n---\n",
        )?;

        let problems = match validate_skill_file(&path) {
            Ok(()) => return Err(Box::new(io::Error::other("name mismatch must fail"))),
            Err(problems) => problems,
        };

        assert!(
            problems.iter().any(|problem| problem
                .contains("`name` is `other-name` but parent directory is `folder-name`")),
            "unexpected problems: {problems:#?}"
        );

        Ok(())
    }

    #[test]
    fn rejects_compatibility_longer_than_spec_limit() -> Result<(), Box<dyn Error>> {
        let path = write_skill_file(
            "compat-skill",
            &format!(
                "---\nname: compat-skill\ndescription: valid description\ncompatibility: \"{}\"\n---\n",
                "y".repeat(COMPATIBILITY_MAX_CHARS + 1)
            ),
        )?;

        let problems = match validate_skill_file(&path) {
            Ok(()) => return Err(Box::new(io::Error::other("compatibility > 500 must fail"))),
            Err(problems) => problems,
        };

        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("`compatibility` length is 501, expected 1-500")),
            "unexpected problems: {problems:#?}"
        );

        Ok(())
    }

    #[test]
    fn extracts_frontmatter_with_crlf_delimiters() -> Result<(), Box<dyn Error>> {
        let frontmatter = extract_frontmatter(
            "---\r\nname: crlf-skill\r\ndescription: works\r\n---\r\n# Body\r\n",
        )
        .map_err(io::Error::other)?;

        assert!(frontmatter.contains("name: crlf-skill"));
        assert!(frontmatter.contains("description: works"));

        Ok(())
    }

    fn write_skill_file(dir_name: &str, content: &str) -> Result<PathBuf, Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let skill_dir = temp_dir.path().join(dir_name);
        fs::create_dir_all(&skill_dir)?;

        let skill_path = skill_dir.join("SKILL.md");
        fs::write(&skill_path, content)?;

        let temp_path = temp_dir.keep();
        assert!(temp_path.exists(), "temp dir should still exist");

        Ok(skill_path)
    }
}
