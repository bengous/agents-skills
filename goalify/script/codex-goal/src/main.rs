use rustix::fd::OwnedFd;
use rustix::fs::{
    ABS, AtFlags, IFlags, Mode, OFlags, fchmod, fchown, fstat, ioctl_getflags, ioctl_setflags,
    mkdirat, openat, unlinkat,
};
use rustix::io::Errno;
use rustix::process::{Gid, Uid, geteuid};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_STDIN_BYTES: usize = 256 * 1024;
const GOALS_DIR: &str = ".codex/goals";
const S_IFMT: u32 = 0o170_000;
const S_IFREG: u32 = 0o100_000;

#[derive(Debug, PartialEq, Eq)]
struct Config {
    root: PathBuf,
    slug: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SudoIdentity {
    uid: u32,
    gid: u32,
}

#[derive(Debug, PartialEq, Eq)]
enum OpenDirError {
    NotFound,
    Other(String),
}

impl OpenDirError {
    fn into_message(self) -> String {
        match self {
            Self::NotFound => "directory not found".to_owned(),
            Self::Other(message) => message,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(output) => match io::stdout().lock().write_all(output.as_bytes()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                let _ = writeln!(io::stderr(), "codex-goal: failed to write stdout: {error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            let _ = writeln!(io::stderr(), "codex-goal: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<String, String> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--version") {
        return Ok(format!("codex-goal {VERSION}\n"));
    }

    let config = parse_args(args)?;
    let identity = sudo_identity_from_env()?;
    let payload = read_payload(io::stdin().lock())?;
    let target = target_path(&config.root, &config.slug);
    write_goal(&config.root, &target, payload.as_bytes(), identity)?;
    success_output(&config.root, &target)
}

fn parse_args(args: Vec<OsString>) -> Result<Config, String> {
    let mut root = None;
    let mut slug = None;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.to_str() {
            Some("--root") => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--root requires an absolute path".to_owned())?;
                let path = PathBuf::from(value);
                if !path.is_absolute() {
                    return Err("--root must be an absolute path".to_owned());
                }
                root = Some(path);
            }
            Some("--slug") => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--slug requires a value".to_owned())?;
                let value = value
                    .into_string()
                    .map_err(|_| "--slug must be valid UTF-8".to_owned())?;
                validate_slug(&value)?;
                slug = Some(value);
            }
            Some("--help" | "-h") => {
                return Err("usage: codex-goal [--root /absolute/root] --slug <slug>".to_owned());
            }
            Some(other) => return Err(format!("unknown argument: {other}")),
            None => return Err("arguments must be valid UTF-8".to_owned()),
        }
    }

    let root = match root {
        Some(path) => path,
        None => env::current_dir().map_err(|error| format!("failed to read cwd: {error}"))?,
    };

    let root = root
        .canonicalize()
        .map_err(|error| format!("failed to canonicalize root {}: {error}", root.display()))?;
    ensure_real_directory(&root, "root")?;
    let cwd = env::current_dir()
        .and_then(|cwd| cwd.canonicalize())
        .map_err(|error| format!("failed to canonicalize cwd: {error}"))?;
    if root != cwd {
        return Err("--root must match the current working directory".to_owned());
    }

    Ok(Config {
        root,
        slug: slug.ok_or_else(|| "--slug is required".to_owned())?,
    })
}

fn validate_slug(slug: &str) -> Result<(), String> {
    if slug.len() > 81 {
        return Err("slug must be at most 81 characters".to_owned());
    }
    if Path::new(slug)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        return Err("slug must not include .md".to_owned());
    }
    if slug.contains("..") {
        return Err("slug must not contain ..".to_owned());
    }
    let Some(first) = slug.chars().next() else {
        return Err("slug must not be empty".to_owned());
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err("slug must start with a lowercase ASCII letter or digit".to_owned());
    }
    if !slug.chars().all(|char| {
        char.is_ascii_lowercase() || char.is_ascii_digit() || matches!(char, '.' | '_' | '-')
    }) {
        return Err("slug must match [a-z0-9][a-z0-9._-]{0,80}".to_owned());
    }
    Ok(())
}

fn sudo_identity_from_env() -> Result<SudoIdentity, String> {
    if geteuid().as_raw() != 0 {
        return Err("must be run as root via sudo".to_owned());
    }
    let uid = parse_env_id("SUDO_UID")?;
    let gid = parse_env_id("SUDO_GID")?;
    if uid == 0 || gid == 0 {
        return Err("SUDO_UID and SUDO_GID must identify a non-root user".to_owned());
    }
    Ok(SudoIdentity { uid, gid })
}

fn parse_env_id(name: &str) -> Result<u32, String> {
    let value = env::var(name).map_err(|_| format!("{name} is required"))?;
    if value.is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    value
        .parse::<u32>()
        .map_err(|_| format!("{name} must be a decimal uid/gid"))
}

fn read_payload<R: Read>(mut reader: R) -> Result<String, String> {
    let mut buffer = Vec::new();
    reader
        .by_ref()
        .take((MAX_STDIN_BYTES + 1) as u64)
        .read_to_end(&mut buffer)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    if buffer.len() > MAX_STDIN_BYTES {
        return Err("stdin payload exceeds 256 KiB".to_owned());
    }
    let payload = String::from_utf8(buffer).map_err(|_| "stdin must be UTF-8".to_owned())?;
    if payload.trim().is_empty() {
        return Err("stdin payload must not be empty".to_owned());
    }
    Ok(payload)
}

fn target_path(root: &Path, slug: &str) -> PathBuf {
    root.join(GOALS_DIR).join(format!("{slug}.md"))
}

fn write_goal(
    root: &Path,
    target: &Path,
    payload: &[u8],
    identity: SudoIdentity,
) -> Result<(), String> {
    let root_fd = open_directory_at(ABS, root, "root").map_err(OpenDirError::into_message)?;
    ensure_fd_user_owned(&root_fd, identity, "root")?;
    let codex_fd = ensure_child_dir(&root_fd, ".codex", identity, ".codex")?;
    let goals_fd = ensure_goals_dir(&codex_fd, identity)?;
    secure_goals_dir(&goals_fd)?;
    set_immutable(&codex_fd, &root.join(".codex"))?;
    verify_path_still_resolves_to_fd(&root_fd, ".codex", &codex_fd, ".codex")?;
    verify_path_still_resolves_to_fd(&codex_fd, "goals", &goals_fd, ".codex/goals")?;

    let file_name = target
        .file_name()
        .ok_or_else(|| "target has no file name".to_owned())?;
    let file = openat(
        &goals_fd,
        file_name,
        OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::RDWR | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o444),
    )
    .map_err(|error| format!("failed to create {}: {error}", target.display()))?;

    let write_result =
        write_payload_and_protect(&goals_fd, file_name, &file, target, payload, identity);
    if let Err(error) = write_result {
        let _ = clear_immutable(&file);
        let _ = unlinkat(&goals_fd, file_name, AtFlags::empty());
        return Err(error);
    }

    Ok(())
}

fn write_payload_and_protect(
    goals_fd: &impl AsFd,
    file_name: &std::ffi::OsStr,
    file: &impl AsFd,
    target: &Path,
    payload: &[u8],
    identity: SudoIdentity,
) -> Result<(), String> {
    let mut std_file = std::fs::File::from(file.as_fd().try_clone_to_owned().map_err(|error| {
        format!(
            "failed to clone file descriptor for {}: {error}",
            target.display()
        )
    })?);
    std_file
        .write_all(payload)
        .map_err(|error| format!("failed to write {}: {error}", target.display()))?;
    std_file
        .sync_all()
        .map_err(|error| format!("failed to sync {}: {error}", target.display()))?;

    fchmod(file, Mode::from_raw_mode(0o444))
        .map_err(|error| format!("failed to chmod {}: {error}", target.display()))?;
    verify_target(file, target, OwnerExpectation::Root)?;
    verify_payload_matches(&mut std_file, target, payload)?;
    assign_owner(file, target, identity)?;
    verify_target(file, target, OwnerExpectation::User(identity))?;
    set_immutable(file, target)?;
    verify_file_path_still_resolves_to_fd(
        goals_fd,
        file_name,
        file,
        &target.display().to_string(),
    )?;
    Ok(())
}

fn clear_immutable(file: &impl AsFd) -> Result<(), Errno> {
    let mut flags = ioctl_getflags(file)?;
    flags.remove(IFlags::IMMUTABLE);
    ioctl_setflags(file, flags)
}

fn assign_owner(file: &impl AsFd, target: &Path, identity: SudoIdentity) -> Result<(), String> {
    fchown(
        file,
        Some(Uid::from_raw(identity.uid)),
        Some(Gid::from_raw(identity.gid)),
    )
    .map_err(|error| {
        format!(
            "failed to assign ownership on {}: {error}",
            target.display()
        )
    })
}

fn set_immutable(file: &impl AsFd, target: &Path) -> Result<(), String> {
    let mut flags = ioctl_getflags(file).map_err(|error| {
        format!(
            "failed to read inode flags for {}: {error}",
            target.display()
        )
    })?;
    flags.insert(IFlags::IMMUTABLE);
    ioctl_setflags(file, flags).map_err(|error| {
        format!(
            "failed to set immutable flag on {}: {error}",
            target.display()
        )
    })?;
    let flags = ioctl_getflags(file).map_err(|error| {
        format!(
            "failed to verify inode flags for {} after immutable write: {error}",
            target.display()
        )
    })?;
    if !flags.contains(IFlags::IMMUTABLE) {
        return Err(format!(
            "immutable flag was not set on {}",
            target.display()
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OwnerExpectation {
    Root,
    User(SudoIdentity),
}

fn verify_target(file: &impl AsFd, target: &Path, owner: OwnerExpectation) -> Result<(), String> {
    let stat =
        fstat(file).map_err(|error| format!("failed to fstat {}: {error}", target.display()))?;
    if stat.st_mode & S_IFMT != S_IFREG {
        return Err(format!(
            "target is not a regular file: {}",
            target.display()
        ));
    }
    match owner {
        OwnerExpectation::Root => {
            if stat.st_uid != 0 || stat.st_gid != 0 {
                return Err(format!("target owner mismatch: {}", target.display()));
            }
        }
        OwnerExpectation::User(identity) => {
            if stat.st_uid != identity.uid || stat.st_gid != identity.gid {
                return Err(format!("target owner mismatch: {}", target.display()));
            }
        }
    }
    if stat.st_mode & 0o777 != 0o444 {
        return Err(format!("target mode is not 0444: {}", target.display()));
    }
    Ok(())
}

fn verify_payload_matches(
    file: &mut std::fs::File,
    target: &Path,
    payload: &[u8],
) -> Result<(), String> {
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("failed to seek {}: {error}", target.display()))?;
    let mut actual = vec![0; payload.len()];
    file.read_exact(&mut actual)
        .map_err(|error| format!("failed to read back {}: {error}", target.display()))?;
    let mut extra = [0; 1];
    let extra_len = file
        .read(&mut extra)
        .map_err(|error| format!("failed to verify eof for {}: {error}", target.display()))?;
    if extra_len != 0 {
        return Err(format!(
            "payload changed before immutable protection: {}",
            target.display()
        ));
    }
    if actual != payload {
        return Err(format!(
            "payload changed before immutable protection: {}",
            target.display()
        ));
    }
    Ok(())
}

fn ensure_child_dir(
    parent: &impl AsFd,
    name: &str,
    identity: SudoIdentity,
    label: &str,
) -> Result<OwnedFd, String> {
    match open_directory_at(parent, name, label) {
        Ok(dir) => {
            ensure_fd_user_owned(&dir, identity, label)?;
            Ok(dir)
        }
        Err(OpenDirError::NotFound) => {
            mkdirat(parent, name, Mode::from_raw_mode(0o755))
                .map_err(|error| format!("failed to create {label}: {error}"))?;
            let dir = open_directory_at(parent, name, label).map_err(OpenDirError::into_message)?;
            fchown(
                &dir,
                Some(Uid::from_raw(identity.uid)),
                Some(Gid::from_raw(identity.gid)),
            )
            .map_err(|error| format!("failed to assign ownership on {label}: {error}"))?;
            ensure_fd_user_owned(&dir, identity, label)?;
            Ok(dir)
        }
        Err(OpenDirError::Other(error)) => Err(error),
    }
}

fn ensure_goals_dir(parent: &impl AsFd, identity: SudoIdentity) -> Result<OwnedFd, String> {
    match open_directory_at(parent, "goals", ".codex/goals") {
        Ok(dir) => {
            ensure_fd_owned_by_user_or_root(&dir, identity, ".codex/goals")?;
            Ok(dir)
        }
        Err(OpenDirError::NotFound) => {
            mkdirat(parent, "goals", Mode::from_raw_mode(0o755))
                .map_err(|error| format!("failed to create .codex/goals: {error}"))?;
            let dir = open_directory_at(parent, "goals", ".codex/goals")
                .map_err(OpenDirError::into_message)?;
            fchown(
                &dir,
                Some(Uid::from_raw(identity.uid)),
                Some(Gid::from_raw(identity.gid)),
            )
            .map_err(|error| format!("failed to assign ownership on .codex/goals: {error}"))?;
            ensure_fd_user_owned(&dir, identity, ".codex/goals")?;
            Ok(dir)
        }
        Err(OpenDirError::Other(error)) => Err(error),
    }
}

fn secure_goals_dir(dir: &impl AsFd) -> Result<(), String> {
    fchown(dir, Some(Uid::from_raw(0)), Some(Gid::from_raw(0)))
        .map_err(|error| format!("failed to assign ownership on .codex/goals: {error}"))?;
    fchmod(dir, Mode::from_raw_mode(0o755))
        .map_err(|error| format!("failed to chmod .codex/goals: {error}"))
}

fn open_directory_at<P: rustix::path::Arg, Fd: AsFd>(
    parent: Fd,
    path: P,
    label: &str,
) -> Result<OwnedFd, OpenDirError> {
    match openat(
        parent,
        path,
        OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(dir) => Ok(dir),
        Err(Errno::NOENT) => Err(OpenDirError::NotFound),
        Err(error) => Err(OpenDirError::Other(format!(
            "{label} must be a real directory: {error}"
        ))),
    }
}

fn ensure_real_directory(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to stat {label} {}: {error}", path.display()))?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return Err(format!("{label} must not be a symlink: {}", path.display()));
    }
    if !file_type.is_dir() {
        return Err(format!("{label} must be a directory: {}", path.display()));
    }
    Ok(())
}

fn ensure_fd_user_owned(
    file: &impl AsFd,
    identity: SudoIdentity,
    label: &str,
) -> Result<(), String> {
    let stat = fstat(file).map_err(|error| format!("failed to fstat {label}: {error}"))?;
    if stat.st_uid != identity.uid || stat.st_gid != identity.gid {
        return Err(format!("{label} must be owned by the invoking user"));
    }
    Ok(())
}

fn ensure_fd_owned_by_user_or_root(
    file: &impl AsFd,
    identity: SudoIdentity,
    label: &str,
) -> Result<(), String> {
    let stat = fstat(file).map_err(|error| format!("failed to fstat {label}: {error}"))?;
    let user_owned = stat.st_uid == identity.uid && stat.st_gid == identity.gid;
    let root_owned = stat.st_uid == 0 && stat.st_gid == 0;
    if !user_owned && !root_owned {
        return Err(format!(
            "{label} must be owned by root or the invoking user"
        ));
    }
    Ok(())
}

fn verify_path_still_resolves_to_fd<P: rustix::path::Arg, ParentFd: AsFd, TargetFd: AsFd>(
    parent: ParentFd,
    path: P,
    target: &TargetFd,
    label: &str,
) -> Result<(), String> {
    let reopened = open_directory_at(parent, path, label).map_err(OpenDirError::into_message)?;
    let target_stat = fstat(target).map_err(|error| format!("failed to fstat {label}: {error}"))?;
    let reopened_stat =
        fstat(&reopened).map_err(|error| format!("failed to fstat reopened {label}: {error}"))?;
    if target_stat.st_dev != reopened_stat.st_dev || target_stat.st_ino != reopened_stat.st_ino {
        return Err(format!("{label} changed while codex-goal was running"));
    }
    Ok(())
}

fn verify_file_path_still_resolves_to_fd<P: rustix::path::Arg, ParentFd: AsFd, TargetFd: AsFd>(
    parent: ParentFd,
    path: P,
    target: &TargetFd,
    label: &str,
) -> Result<(), String> {
    let reopened = openat(
        parent,
        path,
        OFlags::NOFOLLOW | OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| format!("failed to reopen {label}: {error}"))?;
    let target_stat = fstat(target).map_err(|error| format!("failed to fstat {label}: {error}"))?;
    let reopened_stat =
        fstat(&reopened).map_err(|error| format!("failed to fstat reopened {label}: {error}"))?;
    if target_stat.st_dev != reopened_stat.st_dev || target_stat.st_ino != reopened_stat.st_ino {
        return Err(format!("{label} changed while codex-goal was running"));
    }
    Ok(())
}

fn success_output(root: &Path, target: &Path) -> Result<String, String> {
    let display_path = target
        .strip_prefix(root)
        .map_err(|error| format!("failed to make relative output path: {error}"))?
        .display()
        .to_string();

    Ok(format!(
        "Wrote: {display_path}\nProtected: immutable\n\nUsage:\n/goal {display_path}\n"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::os::unix::fs::symlink;

    #[test]
    fn accepts_valid_slug() {
        assert!(validate_slug("improve-goalify_1.2").is_ok());
    }

    #[test]
    fn rejects_invalid_slugs() {
        for slug in [
            "",
            ".hidden",
            "bad slug",
            "bad/slug",
            "bad..slug",
            "bad.md",
            "Échec",
        ] {
            assert!(validate_slug(slug).is_err(), "{slug}");
        }
    }

    #[test]
    fn rejects_relative_root() {
        let result = parse_args(vec![
            "--root".into(),
            "relative".into(),
            "--slug".into(),
            "x".into(),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn reads_non_empty_payload() {
        assert_eq!(
            read_payload(Cursor::new("Objective:\nDo it.\n")),
            Ok("Objective:\nDo it.\n".to_owned())
        );
    }

    #[test]
    fn rejects_empty_payload() {
        assert!(read_payload(Cursor::new(" \n\t")).is_err());
    }

    #[test]
    fn rejects_oversized_payload() {
        let data = vec![b'a'; MAX_STDIN_BYTES + 1];
        assert!(read_payload(Cursor::new(data)).is_err());
    }

    #[test]
    fn builds_target_path() {
        assert_eq!(
            target_path(Path::new("/tmp/work"), "audit"),
            PathBuf::from("/tmp/work/.codex/goals/audit.md")
        );
    }

    #[test]
    fn rejects_symlink_directory_open() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let real = temp.path().join("real");
        let link = temp.path().join("link");
        fs::create_dir_all(&real)?;
        symlink(&real, &link)?;

        assert!(open_directory_at(ABS, &link, "link").is_err());
        Ok(())
    }

    #[test]
    fn exclusive_create_refuses_existing_target() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let existing = temp.path().join("goal.md");
        fs::write(&existing, "existing")?;
        let dir = match open_directory_at(ABS, temp.path(), "temp") {
            Ok(dir) => dir,
            Err(error) => return Err(error.into_message().into()),
        };

        let result = openat(
            &dir,
            "goal.md",
            OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::WRONLY | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o444),
        );

        assert!(result.is_err());
        Ok(())
    }
}
