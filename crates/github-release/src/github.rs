//! GitHub CLI integration. The project uses `gh` instead of a custom API client so authentication matches local and CI usage.

use crate::config::NotesMode;
use crate::domain::ReleasePlan;
use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;
use std::collections::HashMap;
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

#[derive(Clone, Debug)]
pub struct FloatingTagOptions {
    pub stable_line_tags: bool,
    pub latest_tag: bool,
    pub next_tag: bool,
}

impl FloatingTagOptions {
    pub fn for_plan(plan: &ReleasePlan) -> Self {
        Self {
            stable_line_tags: plan.floating_tags,
            latest_tag: plan.latest_tag,
            next_tag: plan.next_tag,
        }
    }

    pub fn with_overrides(
        mut self,
        update_floating_tags: bool,
        update_latest_tag: bool,
        update_next_tag: bool,
    ) -> Self {
        self.stable_line_tags |= update_floating_tags;
        self.latest_tag |= update_latest_tag;
        self.next_tag |= update_next_tag;
        self
    }

    pub fn force_all() -> Self {
        Self {
            stable_line_tags: true,
            latest_tag: true,
            next_tag: true,
        }
    }

    pub fn any(&self) -> bool {
        self.stable_line_tags || self.latest_tag || self.next_tag
    }
}

pub fn refresh_floating_tags_for_plan(
    plan: &ReleasePlan,
    options: FloatingTagOptions,
    dry_run: bool,
) -> Result<()> {
    if !options.any() {
        return Ok(());
    }

    let repository = target_repository_for_tag_updates(plan, dry_run)?;
    let version = Version::parse(&plan.version_text)
        .with_context(|| format!("invalid SemVer version: {}", plan.version_text))?;

    refresh_floating_tags_for_tag(
        &repository,
        &plan.tag,
        &plan.tag_prefix,
        &plan.tag_suffix,
        &plan.latest_tag_name,
        &plan.next_tag_name,
        &version,
        options,
        dry_run,
    )
}

fn target_repository_for_tag_updates(plan: &ReleasePlan, dry_run: bool) -> Result<String> {
    if let Some(repository) = plan.github.target_repository.as_ref() {
        return Ok(repository.clone());
    }

    if dry_run {
        return Ok("<current repository>".to_string());
    }

    current_repository()
}

pub fn refresh_floating_tags_for_tag(
    repository: &str,
    full_tag: &str,
    tag_prefix: &str,
    tag_suffix: &str,
    latest_tag_name: &str,
    next_tag_name: &str,
    version: &Version,
    options: FloatingTagOptions,
    dry_run: bool,
) -> Result<()> {
    if options.stable_line_tags {
        let floating_tags = stable_floating_tags(tag_prefix, tag_suffix, version);
        if floating_tags.is_empty() {
            println!(
                "skipping floating tags for non-stable release tag {full_tag} in {repository}"
            );
        } else {
            let target_sha = if dry_run {
                format!("<resolved target of {full_tag}>")
            } else {
                resolve_tag_target_sha(repository, full_tag)?
            };

            for floating_tag in floating_tags {
                upsert_tag_ref(repository, &floating_tag, &target_sha, dry_run)?;
            }
        }
    }

    if options.latest_tag || options.next_tag {
        refresh_highest_floating_tags(
            repository,
            tag_prefix,
            tag_suffix,
            latest_tag_name,
            next_tag_name,
            FloatingTagOptions {
                stable_line_tags: false,
                latest_tag: options.latest_tag,
                next_tag: options.next_tag,
            },
            dry_run,
        )?;
    }

    Ok(())
}

pub fn refresh_highest_floating_tags(
    repository: &str,
    tag_prefix: &str,
    tag_suffix: &str,
    latest_tag_name: &str,
    next_tag_name: &str,
    options: FloatingTagOptions,
    dry_run: bool,
) -> Result<()> {
    if !options.any() {
        return Ok(());
    }

    let tags = list_matching_tags(repository, tag_prefix)?;
    let analysis = analyze_floating_tags(tags, tag_prefix, tag_suffix);

    if options.stable_line_tags {
        if analysis.highest_major.is_empty() && analysis.highest_minor.is_empty() {
            println!(
                "no stable tags matching {}X.Y.Z{} were found in {repository}",
                tag_prefix, tag_suffix
            );
        }

        for (version, tag) in analysis.highest_minor.values() {
            let target_sha = if dry_run {
                format!("<resolved target of {tag}>")
            } else {
                resolve_tag_target_sha(repository, tag)?
            };
            let floating_tag = format!(
                "{}{}.{}{}",
                tag_prefix, version.major, version.minor, tag_suffix
            );
            upsert_tag_ref(repository, &floating_tag, &target_sha, dry_run)?;
        }

        for (version, tag) in analysis.highest_major.values() {
            let target_sha = if dry_run {
                format!("<resolved target of {tag}>")
            } else {
                resolve_tag_target_sha(repository, tag)?
            };
            let floating_tag = format!("{}{}{}", tag_prefix, version.major, tag_suffix);
            upsert_tag_ref(repository, &floating_tag, &target_sha, dry_run)?;
        }
    }

    if options.latest_tag {
        if let Some((_, tag)) = analysis.latest_stable.as_ref() {
            update_named_floating_tag(repository, latest_tag_name, tag, dry_run)?;
        } else {
            println!("no stable release tags were found for latest tag in {repository}");
        }
    }

    if options.next_tag {
        let target_tag = analysis
            .next_preview
            .as_ref()
            .or(analysis.latest_stable.as_ref());
        if let Some((_, tag)) = target_tag {
            update_named_floating_tag(repository, next_tag_name, tag, dry_run)?;
        } else {
            println!("no preview or stable release tags were found for next tag in {repository}");
        }
    }

    Ok(())
}

fn update_named_floating_tag(
    repository: &str,
    floating_tag: &str,
    target_tag: &str,
    dry_run: bool,
) -> Result<()> {
    let target_sha = if dry_run {
        format!("<resolved target of {target_tag}>")
    } else {
        resolve_tag_target_sha(repository, target_tag)?
    };
    upsert_tag_ref(repository, floating_tag, &target_sha, dry_run)
}

#[derive(Default)]
struct FloatingTagAnalysis {
    highest_major: HashMap<u64, (Version, String)>,
    highest_minor: HashMap<(u64, u64), (Version, String)>,
    latest_stable: Option<(Version, String)>,
    next_preview: Option<(Version, String)>,
}

fn analyze_floating_tags(
    tags: Vec<String>,
    tag_prefix: &str,
    tag_suffix: &str,
) -> FloatingTagAnalysis {
    let mut analysis = FloatingTagAnalysis::default();

    for tag in tags {
        let Some(version) = version_from_tag(&tag, tag_prefix, tag_suffix) else {
            continue;
        };

        if is_stable_patch_version(&version) {
            let major = version.major;
            let minor = version.minor;
            keep_highest(
                &mut analysis.highest_major,
                major,
                version.clone(),
                tag.clone(),
            );
            keep_highest(
                &mut analysis.highest_minor,
                (major, minor),
                version.clone(),
                tag.clone(),
            );
            keep_optional_highest(&mut analysis.latest_stable, version, tag);
        } else {
            keep_optional_highest(&mut analysis.next_preview, version, tag);
        }
    }

    analysis
}

pub fn version_from_tag_for_release(
    tag: &str,
    tag_prefix: &str,
    tag_suffix: &str,
) -> Option<Version> {
    version_from_tag(tag, tag_prefix, tag_suffix)
}

fn version_from_tag(tag: &str, tag_prefix: &str, tag_suffix: &str) -> Option<Version> {
    let version_text = tag.strip_prefix(tag_prefix)?.strip_suffix(tag_suffix)?;
    Version::parse(version_text).ok()
}

pub fn stable_version_from_tag(tag: &str, tag_prefix: &str, tag_suffix: &str) -> Option<Version> {
    let version = version_from_tag(tag, tag_prefix, tag_suffix)?;
    if is_stable_patch_version(&version) {
        Some(version)
    } else {
        None
    }
}

pub fn stable_floating_tags(tag_prefix: &str, tag_suffix: &str, version: &Version) -> Vec<String> {
    if !is_stable_patch_version(version) {
        return Vec::new();
    }

    vec![
        format!(
            "{}{}.{}{}",
            tag_prefix, version.major, version.minor, tag_suffix
        ),
        format!("{}{}{}", tag_prefix, version.major, tag_suffix),
    ]
}

fn is_stable_patch_version(version: &Version) -> bool {
    version.pre.is_empty() && version.build.is_empty()
}

fn keep_optional_highest(target: &mut Option<(Version, String)>, version: Version, tag: String) {
    let should_replace = target
        .as_ref()
        .map(|(current, _)| &version > current)
        .unwrap_or(true);

    if should_replace {
        *target = Some((version, tag));
    }
}

fn keep_highest<K>(map: &mut HashMap<K, (Version, String)>, key: K, version: Version, tag: String)
where
    K: Eq + std::hash::Hash,
{
    let should_replace = map
        .get(&key)
        .map(|(current, _)| &version > current)
        .unwrap_or(true);

    if should_replace {
        map.insert(key, (version, tag));
    }
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
    let body = render_notes_template_with_previous(plan, template, previous_source_tag.as_deref());
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
        .context(
            "source_repository or target_repository is required for generated release notes",
        )?;
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

#[derive(Deserialize)]
struct RefResponse {
    object: GitObject,
}

#[derive(Clone, Deserialize)]
struct GitObject {
    sha: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct TagResponse {
    object: GitObject,
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

fn list_matching_tags(repository: &str, tag_prefix: &str) -> Result<Vec<String>> {
    let endpoint = format!("repos/{repository}/git/matching-refs/tags/{tag_prefix}");
    let output = gh_output(&[
        "api".to_string(),
        "--paginate".to_string(),
        endpoint,
        "--jq".to_string(),
        ".[].ref".to_string(),
    ])?;

    Ok(String::from_utf8_lossy(&output)
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("refs/tags/"))
        .map(ToOwned::to_owned)
        .collect())
}

fn resolve_tag_target_sha(repository: &str, tag: &str) -> Result<String> {
    let endpoint = format!("repos/{repository}/git/ref/tags/{tag}");
    let output = gh_output(&["api".to_string(), endpoint])?;
    let response: RefResponse = serde_json::from_slice(&output)
        .with_context(|| format!("failed to parse tag ref {tag} in {repository}"))?;

    resolve_git_object_to_commit(repository, response.object)
}

fn resolve_git_object_to_commit(repository: &str, mut object: GitObject) -> Result<String> {
    for _ in 0..8 {
        match object.kind.as_str() {
            "commit" => return Ok(object.sha),
            "tag" => {
                let endpoint = format!("repos/{repository}/git/tags/{}", object.sha);
                let output = gh_output(&["api".to_string(), endpoint])?;
                let tag: TagResponse = serde_json::from_slice(&output).with_context(|| {
                    format!(
                        "failed to parse annotated tag object {} in {repository}",
                        object.sha
                    )
                })?;
                object = tag.object;
            }
            kind => anyhow::bail!(
                "tag target in {repository} resolves to unsupported Git object type: {kind}"
            ),
        }
    }

    anyhow::bail!("tag target in {repository} contains too many nested tag objects")
}

fn upsert_tag_ref(repository: &str, tag: &str, target_sha: &str, dry_run: bool) -> Result<()> {
    if dry_run {
        println!(
            "gh api --method PATCH repos/{repository}/git/refs/tags/{tag} -f sha={target_sha} -F force=true"
        );
        return Ok(());
    }

    if tag_ref_exists(repository, tag)? {
        run_gh(
            &[
                "api".to_string(),
                "--method".to_string(),
                "PATCH".to_string(),
                format!("repos/{repository}/git/refs/tags/{tag}"),
                "-f".to_string(),
                format!("sha={target_sha}"),
                "-F".to_string(),
                "force=true".to_string(),
            ],
            false,
        )?;
    } else {
        run_gh(
            &[
                "api".to_string(),
                "--method".to_string(),
                "POST".to_string(),
                format!("repos/{repository}/git/refs"),
                "-f".to_string(),
                format!("ref=refs/tags/{tag}"),
                "-f".to_string(),
                format!("sha={target_sha}"),
            ],
            false,
        )?;
    }

    println!("updated floating tag {repository}@{tag} -> {target_sha}");
    Ok(())
}

fn tag_ref_exists(repository: &str, tag: &str) -> Result<bool> {
    let endpoint = format!("repos/{repository}/git/ref/tags/{tag}");
    let status = Command::new("gh")
        .args(["api", endpoint.as_str()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("failed to inspect tag ref {tag} in {repository}"))?;

    Ok(status.success())
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

fn gh_output(args: &[String]) -> Result<Vec<u8>> {
    let output = Command::new("gh")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("failed to run gh {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh command failed: gh {}\n{stderr}", args.join(" "));
    }

    Ok(output.stdout)
}

fn current_repository() -> Result<String> {
    let output = gh_output(&[
        "repo".to_string(),
        "view".to_string(),
        "--json".to_string(),
        "nameWithOwner".to_string(),
        "--jq".to_string(),
        ".nameWithOwner".to_string(),
    ])?;
    let repository = String::from_utf8_lossy(&output).trim().to_string();
    if repository.is_empty() {
        anyhow::bail!("failed to resolve current GitHub repository");
    }
    Ok(repository)
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
            tag_prefix: "v".to_string(),
            tag_suffix: String::new(),
            release_name: "tool v0.1.0".to_string(),
            target_branch: "master".to_string(),
            release_branch: "release/v0.1.0".to_string(),
            prerelease: false,
            latest: true,
            commit_message: "prepare v0.1.0".to_string(),
            merge_message: "merge v0.1.0".to_string(),
            floating_tags: false,
            latest_tag: false,
            next_tag: false,
            latest_tag_name: "latest".to_string(),
            next_tag_name: "next".to_string(),
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
    fn stable_floating_tags_keep_release_prefix_and_suffix() {
        let version = Version::parse("1.2.3").expect("version");

        assert_eq!(
            stable_floating_tags("action-v", "-dist", &version),
            vec!["action-v1.2-dist".to_string(), "action-v1-dist".to_string()]
        );
    }

    #[test]
    fn stable_floating_tags_skip_prereleases_and_build_metadata() {
        assert!(
            stable_floating_tags("v", "", &Version::parse("1.2.3-rc.1").expect("version"))
                .is_empty()
        );
        assert!(
            stable_floating_tags("v", "", &Version::parse("1.2.3+build.1").expect("version"))
                .is_empty()
        );
    }

    #[test]
    fn stable_version_from_tag_requires_full_patch_tag() {
        assert_eq!(
            stable_version_from_tag("action-v1.2.3-dist", "action-v", "-dist")
                .expect("version")
                .to_string(),
            "1.2.3"
        );
        assert!(stable_version_from_tag("action-v1.2-dist", "action-v", "-dist").is_none());
        assert!(stable_version_from_tag("action-v1.2.3-rc.1-dist", "action-v", "-dist").is_none());
    }

    #[test]
    fn floating_tag_analysis_finds_latest_stable_and_next_preview() {
        let analysis = analyze_floating_tags(
            vec![
                "v1.0.0".to_string(),
                "v1.1.0".to_string(),
                "v1.2.0-rc.1".to_string(),
                "v2.0.0-alpha.1".to_string(),
                "v1.1".to_string(),
                "latest".to_string(),
            ],
            "v",
            "",
        );

        assert_eq!(analysis.latest_stable.expect("latest stable").1, "v1.1.0");
        assert_eq!(
            analysis.next_preview.expect("next preview").1,
            "v2.0.0-alpha.1"
        );
    }

    #[test]
    fn version_from_tag_for_release_keeps_prerelease_tags() {
        assert_eq!(
            version_from_tag_for_release("tool-v1.2.3-rc.1-dist", "tool-v", "-dist")
                .expect("version")
                .to_string(),
            "1.2.3-rc.1"
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
