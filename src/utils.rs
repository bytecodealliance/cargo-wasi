use crate::config::Config;
use anyhow::{anyhow, bail, Context, Error, Result};
use fs2::FileExt;
use std::fmt;
use std::fs;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::process::{Command, ExitStatus, Output, Stdio};

pub trait CommandExt {
    fn as_command_mut(&mut self) -> &mut Command;

    fn capture_stdout(&mut self) -> Result<String> {
        let cmd = self.as_command_mut();
        let output = cmd.stderr(Stdio::inherit()).output_if_success()?;
        let s = String::from_utf8(output.stdout)
            .map_err(|_| anyhow!("process output was not utf-8"))
            .with_context(|| format!("failed to execute {:?}", cmd))?;
        Ok(s)
    }

    fn run(&mut self) -> Result<()> {
        let cmd = self.as_command_mut();
        cmd.stderr(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stdin(Stdio::inherit())
            .output_if_success()?;
        Ok(())
    }

    fn output_if_success(&mut self) -> Result<Output> {
        let cmd = self.as_command_mut();
        let output = cmd
            .output()
            .with_context(|| format!("failed to create process {:?}", cmd))?;
        check_success(cmd, &output.status, &output.stdout, &output.stderr)?;
        Ok(output)
    }
}

impl CommandExt for Command {
    fn as_command_mut(&mut self) -> &mut Command {
        self
    }
}

pub fn check_success(
    cmd: &Command,
    status: &ExitStatus,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<()> {
    if status.success() {
        return Ok(());
    }
    Err(ProcessError {
        cmd_desc: format!("{:?}", cmd),
        status: status.clone(),
        stdout: stdout.to_vec(),
        stderr: stderr.to_vec(),
        hidden: false,
    }
    .into())
}

pub fn flock(path: &Path) -> Result<impl Drop> {
    struct Lock(File);
    let parent = path.parent().unwrap();
    fs::create_dir_all(parent)
        .context(format!("failed to create directory `{}`", parent.display()))?;
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)?;
    file.lock_exclusive()?;
    return Ok(Lock(file));

    impl Drop for Lock {
        fn drop(&mut self) {
            drop(self.0.unlock());
        }
    }
}

/// If `Error` is a `ProcessError` and it looks like a "normal exit", then it
/// flags that the `ProcessError` will be hidden.
///
/// Hidden errors won't get printed at the top-level as they propagate outwards
/// since it's trusted that the relevant program printed out all the relevant
/// information.
pub fn hide_normal_process_exit(error: Error, config: &Config) -> Error {
    if config.is_verbose() {
        return error;
    }
    let mut error = match error.downcast::<ProcessError>() {
        Ok(e) => e,
        Err(e) => return e,
    };
    if let Some(code) = error.status.code() {
        if 0 <= code && code < 128 && error.stdout.is_empty() && error.stderr.is_empty() {
            error.hidden = true;
        }
    }
    error.into()
}

/// Checks if `Error` has been hidden via `hide_normal_process_exit` above.
pub fn normal_process_exit_code(error: &Error) -> Option<i32> {
    let process_error = error.downcast_ref::<ProcessError>()?;
    if !process_error.hidden {
        return None;
    }
    process_error.status.code()
}

#[derive(Debug)]
struct ProcessError {
    status: ExitStatus,
    hidden: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    cmd_desc: String,
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to execute {}", self.cmd_desc)?;
        write!(f, "\n    status: {}", self.status)?;
        if !self.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&self.stdout);
            let stdout = stdout.replace("\n", "\n        ");
            write!(f, "\n    stdout:\n        {}", stdout)?;
        }
        if !self.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&self.stderr);
            let stderr = stderr.replace("\n", "\n        ");
            write!(f, "\n    stderr:\n        {}", stderr)?;
        }
        Ok(())
    }
}

impl std::error::Error for ProcessError {}

pub fn get(url: &str) -> Result<reqwest::Response> {
    let response = reqwest::get(url).context(format!("failed to fetch {}", url))?;
    if !response.status().is_success() {
        bail!(
            "failed to get successful response from {}: {}",
            url,
            response.status()
        );
    }
    Ok(response)
}
