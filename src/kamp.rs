mod argv;
mod cmd;
mod context;
mod error;
mod kak;

use argv::{Kampliment, SubCommand::*};
use context::Context;
use error::Error;

const KAKOUNE_SESSION: &str = "KAKOUNE_SESSION";
const KAKOUNE_CLIENT: &str = "KAKOUNE_CLIENT";

pub(super) fn run() -> Result<Option<String>, Error> {
    let kamp: Kampliment = argh::from_env();
    let (session, client) = match (kamp.session, kamp.client) {
        (Some(s), client) => (Some(s), client),
        (None, client) => (
            std::env::var(KAKOUNE_SESSION).ok(),
            client.or_else(|| std::env::var(KAKOUNE_CLIENT).ok()),
        ),
    };

    let ctx = session
        .map(|session| Context::new(session, client.as_deref()))
        .ok_or(Error::NoSession);

    match kamp.subcommand {
        Init(opt) => cmd::init(opt.export, opt.alias).map(Some),
        Attach(opt) => cmd::attach(&ctx?, opt.buffer).map(|_| None),
        Edit(opt) => {
            if let Ok(ctx) = ctx {
                cmd::edit(&ctx, opt.files).map(|_| None)
            } else {
                kak::proxy(opt.files).map_err(Error::Other).map(|_| None)
            }
        }
        Send(opt) => cmd::send(
            &ctx?,
            join_command(opt.command, opt.remainder),
            to_csv_buffers_or_asterisk(opt.buffers),
        )
        .map(|_| None),
        List(opt) => {
            if opt.all {
                cmd::list_all(ctx.ok()).map(Some)
            } else {
                cmd::list(&ctx?).map(Some)
            }
        }
        Kill(opt) => cmd::kill(&ctx?, opt.exit_status).map(|_| None),
        Get(opt) => cmd::Get::from(opt.subcommand)
            .run(&ctx?, opt.raw, to_csv_buffers_or_asterisk(opt.buffers))
            .map(Some),
        Cat(opt) => cmd::cat(&ctx?, to_csv_buffers_or_asterisk(opt.buffers)).map(Some),
        Ctx(_) => cmd::ctx(&ctx?).map(Some),
        Version(_) => cmd::version().map(Some),
    }
}

fn join_command(cmd: String, remainder: Vec<String>) -> String {
    remainder.into_iter().fold(cmd, |mut cmd, next| {
        cmd.push(' ');
        cmd.push_str(&next);
        cmd
    })
}

fn to_csv_buffers_or_asterisk(buffers: Vec<String>) -> Option<String> {
    if buffers.is_empty() {
        return None;
    }
    if buffers[0] == "*" {
        return buffers.into_iter().rev().last();
    }
    let mut res =
        buffers
            .into_iter()
            .filter(|s| s != "*")
            .fold(String::from('\''), |mut buf, next| {
                buf.push_str(&next);
                buf.push(',');
                buf
            });
    res.pop(); // pops last ','
    res.push('\'');
    Some(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_to_csv_buffers_or_asterisk() {
        assert_eq!(to_csv_buffers_or_asterisk(vec![]), None);
        assert_eq!(
            to_csv_buffers_or_asterisk(vec!["*".into()]),
            Some("*".into())
        );
        assert_eq!(
            to_csv_buffers_or_asterisk(vec!["*".into(), "a".into()]),
            Some("*".into())
        );
        assert_eq!(
            to_csv_buffers_or_asterisk(vec!["a".into(), "*".into()]),
            Some("'a'".into())
        );
        assert_eq!(
            to_csv_buffers_or_asterisk(vec!["a".into(), "b".into()]),
            Some("'a,b'".into())
        );
        assert_eq!(
            to_csv_buffers_or_asterisk(vec!["a".into(), "b".into(), "c".into()]),
            Some("'a,b,c'".into())
        );
    }
}
