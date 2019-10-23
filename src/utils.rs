use anyhow::{anyhow, bail, Context, Result};
use fs2::FileExt;
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
    let mut message = format!("failed to execute {:?}", cmd);
    message.push_str(&format!("\n\tstatus: {}", status));
    if !stdout.is_empty() {
        let stdout = String::from_utf8_lossy(&stdout);
        let stdout = stdout.replace("\n", "\n\t\t");
        message.push_str(&format!("\n\tstdout:\n\t\t{}", stdout));
    }
    if !stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&stderr);
        let stderr = stderr.replace("\n", "\n\t\t");
        message.push_str(&format!("\n\tstderr:\n\t\t{}", stderr));
    }
    bail!("{}", message);
}

pub fn flock(path: &Path) -> Result<impl Drop> {
    struct Lock(File);
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
