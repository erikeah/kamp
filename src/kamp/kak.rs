use std::io::{Error, ErrorKind, Result, Write};
use std::process::{Command, ExitStatus, Stdio};

pub(crate) fn list_sessions() -> Result<Vec<u8>> {
    let output = Command::new("kak").arg("-l").output()?;

    if !output.status.success() {
        return Err(match output.status.code() {
            Some(code) => Error::new(
                ErrorKind::Other,
                format!("kak exited with status code: {code}"),
            ),
            None => Error::new(ErrorKind::Other, "kak terminated by signal"),
        });
    }

    Ok(output.stdout)
}

pub(crate) fn pipe<S, T>(session: S, cmd: T) -> Result<ExitStatus>
where
    S: AsRef<str>,
    T: AsRef<[u8]>,
{
    let mut child = Command::new("kak")
        .arg("-p")
        .arg(session.as_ref())
        .stdin(Stdio::piped())
        .spawn()?;

    let Some(stdin) = child.stdin.as_mut() else {
        return Err(Error::new(
            ErrorKind::Other,
            "cannot capture stdin of kak process",
        ));
    };

    stdin.write_all(cmd.as_ref()).and_then(|_| child.wait())
}

pub(crate) fn connect<S: AsRef<str>>(session: S, cmd: S) -> Result<ExitStatus> {
    Command::new("kak")
        .arg("-c")
        .arg(session.as_ref())
        .arg("-e")
        .arg(cmd.as_ref())
        .status()
}

pub(crate) fn proxy(args: Vec<String>) -> Result<()> {
    use std::os::unix::prelude::CommandExt;
    Err(Command::new("kak").args(args).exec())
}
