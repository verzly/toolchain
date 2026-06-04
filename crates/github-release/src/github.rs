//! GitHub CLI integration. The project uses `gh` instead of a custom API client so authentication matches local and CI usage.

use crate::config::NotesMode;
use crate::domain::ReleasePlan;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct ReleaseNotesInput<'a> {
    pub body: Option<&'a str>,
    pub file: Option<&'a Path>,
}

pub fn create_release(
    plan: &ReleasePlan,
    assets_dir: Option<&Path>,
    notes: ReleaseNotesInput<'_>,
    dry_run: bool,
) -> Result<()> {
    let notes_file = release_notes_file(plan, notes, dry_run)?;

    let mut args = vec![
        "release".to_string(),
        "create".to_string(),
        plan.tag.clone(),
        "--title".to_string(),
        plan.release_name.clone(),
        "--target".to_string(),
        plan.target_branch.clone(),
    ];

    if let Some(repository) = plan.github.target_repository.as_ref() {
        args.push("--repo".to_string());
        args.push(repository.clone());
    }

    if let Some(path) = notes_file.as_ref() {
        args.push("--notes-file".to_string());
        args.push(path.display().to_string());
    } else if plan.github.generate_notes {
        args.push("--generate-notes".to_string());
    }

    if plan.prerelease {
        args.push("--prerelease".to_string());
    }

    if !plan.latest {
        args.push("--latest=false".to_string());
    }

    run_gh(&args, dry_run)?;

    if let Some(dir) = assets_dir {
        let assets = collect_assets(dir)?;
        if !assets.is_empty() {
            let mut upload_args = vec![
                "release".to_string(),
                "upload".to_string(),
                plan.tag.clone(),
            ];
            if let Some(repository) = plan.github.target_repository.as_ref() {
                upload_args.push("--repo".to_string());
                upload_args.push(repository.clone());
            }
            upload_args.extend(assets.iter().map(|path| path.display().to_string()));
            upload_args.push("--clobber".to_string());
            run_gh(&upload_args, dry_run)?;
        }
    }

    Ok(())
}

fn release_notes_file(
    plan: &ReleasePlan,
    notes: ReleaseNotesInput<'_>,
    dry_run: bool,
) -> Result<Option<PathBuf>> {
    if let Some(path) = notes.file {
        return Ok(Some(path.to_path_buf()));
    }

    if let Some(body) = notes.body {
        return Ok(Some(write_custom_notes_file(plan, body)?));
    }

    if !plan.github.notes_body.trim().is_empty() {
        return Ok(Some(write_custom_notes_file(
            plan,
            &plan.github.notes_body,
        )?));
    }

    if plan.github.generate_notes
        && (plan.github.source_repository.is_some() || plan.github.target_repository.is_some())
    {
        return Ok(Some(write_generated_notes_file(plan, dry_run)?));
    }

    Ok(None)
}

fn write_custom_notes_file(plan: &ReleasePlan, template: &str) -> Result<PathBuf> {
    let body = render_notes_template(plan, template)?;
    let path = std::env::temp_dir().join(format!("github-release-{}-custom-notes.md", plan.tag));
    fs::write(&path, body)
        .with_context(|| format!("failed to write release notes file {}", path.display()))?;
    Ok(path)
}

fn render_notes_template(plan: &ReleasePlan, template: &str) -> Result<String> {
    let previous_source_tag = previous_source_tag(plan)?;
    let body = render_notes_template_with_previous(
        plan,
        template,
        previous_source_tag.as_deref(),
    );
    Ok(normalize_release_note_links(&body, plan))
}

fn render_notes_template_with_previous(
    plan: &ReleasePlan,
    template: &str,
    previous_source_tag: Option<&str>,
) -> String {
    let previous = previous_source_tag.unwrap_or("");
    let compare_url = source_compare_url(plan, previous_source_tag);
    let source_repository = plan.github.source_repository.as_deref().unwrap_or("");

    template
        .replace("{version}", &plan.version_text)
        .replace("{tag}", &plan.tag)
        .replace("{release_name}", &plan.release_name)
        .replace(
            "{target_repository}",
            plan.github.target_repository.as_deref().unwrap_or(""),
        )
        .replace("{source_repository}", source_repository)
        .replace("{source_tag}", &plan.github.source_tag)
        .replace("{previous_source_tag}", previous)
        .replace("{source_compare_url}", &compare_url)
}

fn source_compare_url(plan: &ReleasePlan, previous_source_tag: Option<&str>) -> String {
    let Some(repository) = plan.github.source_repository.as_ref() else {
        return String::new();
    };

    match previous_source_tag {
        Some(previous) => format!(
            "https://github.com/{repository}/compare/{previous}...{}",
            plan.github.source_tag
        ),
        None => format!(
            "https://github.com/{repository}/releases/tag/{}",
            plan.github.source_tag
        ),
    }
}

fn normalize_release_note_links(body: &str, plan: &ReleasePlan) -> String {
    normalize_pull_request_links(body, plan.github.target_repository.as_deref())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PullRequestUrl<'a> {
    end: usize,
    owner: &'a str,
    repo: &'a str,
    number: &'a str,
}

fn normalize_pull_request_links(body: &str, context_repository: Option<&str>) -> String {
    const PREFIX: &str = "https://github.com/";

    let mut normalized = String::with_capacity(body.len());
    let mut cursor = 0;

    while let Some(relative_start) = body[cursor..].find(PREFIX) {
        let start = cursor + relative_start;
        normalized.push_str(&body[cursor..start]);

        let Some(url) = parse_pull_request_url(&body[start..]) else {
            normalized.push_str(PREFIX);
            cursor = start + PREFIX.len();
            continue;
        };

        let end = start + url.end;
        if is_markdown_link_destination(body, start) {
            normalized.push_str(&body[start..end]);
        } else {
            let repository = format!("{}/{}", url.owner, url.repo);
            let label = if same_repository(context_repository, &repository) {
                format!("#{}", url.number)
            } else {
                format!("{}#{}", url.repo, url.number)
            };
            normalized.push_str(&format!("[{label}]({})", &body[start..end]));
        }
        cursor = end;
    }

    normalized.push_str(&body[cursor..]);
    normalized
}

fn parse_pull_request_url(value: &str) -> Option<PullRequestUrl<'_>> {
    const PREFIX: &str = "https://github.com/";
    let rest = value.strip_prefix(PREFIX)?;

    let owner_end = rest.find('/')?;
    let owner = &rest[..owner_end];
    let rest = &rest[owner_end + 1..];

    let repo_end = rest.find("/pull/")?;
    let repo = &rest[..repo_end];
    let rest = &rest[repo_end + "/pull/".len()..];

    let number_len = rest
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_digit())
        .last()
        .map(|(index, ch)| index + ch.len_utf8())?;

    if owner.is_empty() || repo.is_empty() || number_len == 0 {
        return None;
    }

    Some(PullRequestUrl {
        end: PREFIX.len() + owner.len() + 1 + repo.len() + "/pull/".len() + number_len,
        owner,
        repo,
        number: &rest[..number_len],
    })
}

fn is_markdown_link_destination(body: &str, url_start: usize) -> bool {
    body[..url_start].ends_with("](")
}

fn same_repository(context_repository: Option<&str>, repository: &str) -> bool {
    context_repository
        .map(|value| value.eq_ignore_ascii_case(repository))
        .unwrap_or(false)
}

fn write_generated_notes_file(plan: &ReleasePlan, dry_run: bool) -> Result<PathBuf> {
    let notes_repository = plan
        .github
        .source_repository
        .as_ref()
        .or(plan.github.target_repository.as_ref())
        .context("source_repository or target_repository is required for generated release notes")?;
    let notes_tag = if plan.github.source_repository.is_some() {
        &plan.github.source_tag
    } else {
        &plan.tag
    };
    let target_commitish = if plan.github.source_repository.is_some() {
        None
    } else {
        Some(plan.target_branch.as_str())
    };

    let generated = if dry_run {
        format!(
            "Generated release notes would be requested from `{notes_repository}` for `{notes_tag}`."
        )
    } else if plan.github.source_repository.is_some() && plan.github.notes.mode == NotesMode::Scoped
    {
        generate_scoped_notes_from_git(plan)
            .unwrap_or_else(|error| fallback_notes(notes_repository, notes_tag, &error.to_string()))
    } else {
        generate_notes_from_repository(notes_repository, notes_tag, target_commitish)
            .unwrap_or_else(|error| fallback_notes(notes_repository, notes_tag, &error.to_string()))
    };
    let generated = normalize_release_note_links(&generated, plan);

    let mut body = String::new();
    body.push_str(&generated);

    if let Some(source_repository) = plan.github.source_repository.as_ref() {
        body.push_str("\n\n---\n\n");
        body.push_str(&format!(
            "Source changes for this release are maintained in `{source_repository}`. Pull request links in \
             these notes intentionally point to that source repository, even when the release itself is \
             published from a distribution repository.\n"
        ));
    }

    let path = std::env::temp_dir().join(format!("github-release-{}-notes.md", plan.tag));
    fs::write(&path, body)
        .with_context(|| format!("failed to write release notes file {}", path.display()))?;
    Ok(path)
}

#[derive(Deserialize)]
struct GeneratedNotes {
    body: Option<String>,
}

fn generate_notes_from_repository(
    repository: &str,
    tag: &str,
    target_commitish: Option<&str>,
) -> Result<String> {
    let endpoint = format!("repos/{repository}/releases/generate-notes");
    let mut args = vec![
        "api".to_string(),
        endpoint,
        "-f".to_string(),
        format!("tag_name={tag}"),
    ];
    if let Some(target_commitish) = target_commitish {
        args.push("-f".to_string());
        args.push(format!("target_commitish={target_commitish}"));
    }

    let output = Command::new("gh")
        .args(&args)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("failed to generate release notes from {repository}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh api failed while generating release notes from {repository}: {stderr}");
    }

    let notes: GeneratedNotes = serde_json::from_slice(&output.stdout)
        .context("failed to parse generated release notes response")?;
    Ok(notes.body.unwrap_or_default())
}

#[derive(Clone, Debug)]
struct CommitEntry {
    hash: String,
    subject: String,
    paths: Vec<String>,
}

fn generate_scoped_notes_from_git(plan: &ReleasePlan) -> Result<String> {
    let previous_tag = previous_source_tag(plan)?;
    let range = previous_tag
        .as_ref()
        .map(|tag| format!("{tag}..{}", plan.github.source_tag))
        .unwrap_or_else(|| plan.github.source_tag.clone());
    let commits = commits_in_range(&range)?;
    let include_scopes = normalized_values(&plan.github.notes.include_scopes);
    let include_paths = normalized_paths(&plan.github.notes.include_paths);

    let mut included = Vec::new();
    for commit in commits {
        if commit_matches(&commit, &include_scopes, &include_paths) {
            included.push(commit);
        }
    }

    let mut body = String::new();
    body.push_str("## What's changed\n\n");

    if included.is_empty() {
        body.push_str("No package-specific changes were detected for this release.\n");
    } else {
        for commit in included {
            let short_hash = commit.hash.chars().take(7).collect::<String>();
            body.push_str(&format!("- {} (`{short_hash}`)\n", commit.subject));
        }
    }

    body.push('\n');
    if let Some(previous_tag) = previous_tag {
        body.push_str(&format!(
            "Compared source tags: `{previous_tag}` -> `{}`.\n",
            plan.github.source_tag
        ));
    } else {
        body.push_str(&format!(
            "Compared source tag: `{}`. No earlier matching source tag was found.\n",
            plan.github.source_tag
        ));
    }

    Ok(body)
}

fn previous_source_tag(plan: &ReleasePlan) -> Result<Option<String>> {
    let pattern = format!(
        "{}*{}",
        plan.github.source_tag_prefix, plan.github.source_tag_suffix
    );
    let output = git_output(["tag", "--list", &pattern, "--sort=-creatordate"])?;
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .find(|tag| *tag != plan.github.source_tag)
        .map(ToOwned::to_owned))
}

fn commits_in_range(range: &str) -> Result<Vec<CommitEntry>> {
    let output = git_output(["log", "--format=%H%x1f%s", range])?;
    let mut commits = Vec::new();

    for line in output.lines() {
        let Some((hash, subject)) = line.split_once('\u{1f}') else {
            continue;
        };
        commits.push(CommitEntry {
            hash: hash.to_string(),
            subject: subject.to_string(),
            paths: commit_paths(hash)?,
        });
    }

    Ok(commits)
}

fn commit_paths(hash: &str) -> Result<Vec<String>> {
    let output = git_output(["show", "--format=", "--name-only", "--no-renames", hash])?;
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn commit_matches(
    commit: &CommitEntry,
    include_scopes: &[String],
    include_paths: &[String],
) -> bool {
    if let Some(scope) = commit_scope(&commit.subject) {
        let normalized = scope.to_ascii_lowercase();
        if normalized == "all" || include_scopes.iter().any(|scope| scope == &normalized) {
            return true;
        }
    }

    commit.paths.iter().any(|path| {
        let normalized = normalize_path(path);
        include_paths
            .iter()
            .any(|prefix| normalized.starts_with(prefix))
    })
}

fn commit_scope(subject: &str) -> Option<&str> {
    let open = subject.find('(')?;
    let close = subject[open + 1..].find(')')? + open + 1;
    let suffix = subject.get(close + 1..)?;
    if suffix.starts_with(':') || suffix.starts_with("!:") {
        Some(&subject[open + 1..close])
    } else {
        None
    }
}

fn normalized_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn normalized_paths(paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .map(|path| normalize_path(path))
        .filter(|path| !path.is_empty())
        .collect()
}

fn normalize_path(path: &str) -> String {
    path.trim().trim_start_matches("./").replace('\\', "/")
}

fn git_output<const N: usize>(args: [&str; N]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .context("failed to run git command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git command failed: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn fallback_notes(repository: &str, tag: &str, error: &str) -> String {
    // External release notes are a convenience, not a reason to block publishing.
    // The fallback keeps the public release honest when the source repository is private or GitHub cannot generate notes.
    format!(
        "What's changed\n\nRelease notes could not be generated automatically from `{repository}` for \
         `{tag}`.\n\nReason: {error}\n"
    )
}

fn run_gh(args: &[String], dry_run: bool) -> Result<()> {
    if dry_run {
        println!("gh {}", args.join(" "));
        return Ok(());
    }

    let status = Command::new("gh")
        .args(args)
        .stdin(Stdio::null())
        .status()
        .with_context(|| format!("failed to run gh {}", args.join(" ")))?;

    if !status.success() {
        anyhow::bail!("gh command failed: gh {}", args.join(" "));
    }

    Ok(())
}

fn collect_assets(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        anyhow::bail!("asset directory does not exist: {}", dir.display());
    }

    let mut assets = Vec::new();
    collect_assets_recursive(dir, &mut assets)?;
    assets.sort();
    Ok(assets)
}

fn collect_assets_recursive(dir: &Path, assets: &mut Vec<PathBuf>) -> Result<()> {
    // cargo-release groups artifacts by target under dist/. GitHub Releases use
    // the file name as the asset name, so nested target folders are safe here.
    for entry in
        std::fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))?
    {
        let path = entry?.path();
        if path.is_dir() {
            collect_assets_recursive(&path, assets)?;
        } else if path.is_file() {
            assets.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release_plan() -> ReleasePlan {
        ReleasePlan {
            version_text: "0.1.0".to_string(),
            tag: "v0.1.0".to_string(),
            release_name: "tool v0.1.0".to_string(),
            target_branch: "master".to_string(),
            release_branch: "release/v0.1.0".to_string(),
            prerelease: false,
            latest: true,
            commit_message: "prepare v0.1.0".to_string(),
            merge_message: "merge v0.1.0".to_string(),
            github: crate::domain::GitHubPlan {
                target_repository: Some("verzly/tool".to_string()),
                source_repository: Some("verzly/toolchain".to_string()),
                source_tag: "tool-v0.1.0".to_string(),
                source_tag_prefix: "tool-v".to_string(),
                source_tag_suffix: String::new(),
                generate_notes: false,
                notes_body: String::new(),
                notes: crate::domain::NotesPlan {
                    mode: NotesMode::Scoped,
                    include_scopes: vec!["tool".to_string()],
                    include_paths: vec!["crates/tool/".to_string()],
                },
            },
        }
    }

    #[test]
    fn custom_notes_template_renders_source_compare_link() {
        let plan = release_plan();
        let body = render_notes_template_with_previous(
            &plan,
            "Developed in {source_repository} from {previous_source_tag} to {source_tag}: {source_compare_url}",
            Some("tool-v0.0.9"),
        );

        assert_eq!(
            body,
            "Developed in verzly/toolchain from tool-v0.0.9 to tool-v0.1.0: https://github.com/verzly/toolchain/compare/tool-v0.0.9...tool-v0.1.0"
        );
    }

    #[test]
    fn custom_notes_template_falls_back_to_source_tag_link_without_previous_tag() {
        let plan = release_plan();
        let body = render_notes_template_with_previous(
            &plan,
            "Source changes: {source_compare_url}",
            None,
        );

        assert_eq!(
            body,
            "Source changes: https://github.com/verzly/toolchain/releases/tag/tool-v0.1.0"
        );
    }

    #[test]
    fn commit_scope_reads_conventional_commit_scope() {
        assert_eq!(
            commit_scope("feat(github-release): add notes"),
            Some("github-release")
        );
        assert_eq!(
            commit_scope("feat(github-release)!: change tags"),
            Some("github-release")
        );
        assert_eq!(commit_scope("fix: unscoped change"), None);
        assert_eq!(commit_scope("docs(readme) update missing colon"), None);
    }

    #[test]
    fn commit_matching_accepts_scope_all_and_configured_paths() {
        let include_scopes = ["cargo-release".to_string()];
        let include_paths = ["crates/cargo-release/".to_string()];

        let scoped = CommitEntry {
            hash: "abc".to_string(),
            subject: "fix(cargo-release): correct artifact name".to_string(),
            paths: vec!["README.md".to_string()],
        };
        let all = CommitEntry {
            hash: "def".to_string(),
            subject: "chore(all): update release workflows".to_string(),
            paths: vec![".github/workflows/test.yml".to_string()],
        };
        let path_matched = CommitEntry {
            hash: "ghi".to_string(),
            subject: "refactor: move helper".to_string(),
            paths: vec!["./crates/cargo-release/src/artifacts.rs".to_string()],
        };
        let unrelated = CommitEntry {
            hash: "jkl".to_string(),
            subject: "fix(rust-cache): update env planning".to_string(),
            paths: vec!["crates/rust-cache/src/main.rs".to_string()],
        };

        assert!(commit_matches(&scoped, &include_scopes, &include_paths));
        assert!(commit_matches(&all, &include_scopes, &include_paths));
        assert!(commit_matches(
            &path_matched,
            &include_scopes,
            &include_paths
        ));
        assert!(!commit_matches(&unrelated, &include_scopes, &include_paths));
    }

    #[test]
    fn release_note_link_normalization_hides_pull_request_urls() {
        let body = "## What's changed

- Fix cache path in https://github.com/verzly/toolchain/pull/15.
- Keep public docs in https://github.com/verzly/cargo-release/pull/9";

        let normalized = normalize_pull_request_links(body, Some("verzly/cargo-release"));

        assert_eq!(
            normalized,
            "## What's changed

- Fix cache path in [toolchain#15](https://github.com/verzly/toolchain/pull/15).
- Keep public docs in [#9](https://github.com/verzly/cargo-release/pull/9)"
        );
    }

    #[test]
    fn release_note_link_normalization_preserves_existing_markdown_destinations() {
        let body = "See [toolchain#15](https://github.com/verzly/toolchain/pull/15).";

        assert_eq!(
            normalize_pull_request_links(body, Some("verzly/cargo-release")),
            body
        );
    }

    #[test]
    fn normalizes_windows_and_relative_paths() {
        assert_eq!(
            normalize_path("./crates\\cargo-release\\src\\main.rs"),
            "crates/cargo-release/src/main.rs"
        );
        assert_eq!(
            normalize_path(" crates/cargo-release/src/main.rs "),
            "crates/cargo-release/src/main.rs"
        );
    }
}
