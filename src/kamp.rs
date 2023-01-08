mod argv;
mod cmd;
mod context;
mod error;
mod kak;

use argv::{Kampliment, SubCommand::*};
use context::Context;
use error::{Error, Result};

const KAKOUNE_SESSION: &str = "KAKOUNE_SESSION";
const KAKOUNE_CLIENT: &str = "KAKOUNE_CLIENT";

pub(super) fn run() -> Result<Option<String>> {
    let kamp: Kampliment = argh::from_env();
    let (session, client) = match (kamp.session, kamp.client) {
        (Some(s), client) => (Some(s), client),
        (None, client) => (
            std::env::var(KAKOUNE_SESSION).ok(),
            client.or_else(|| std::env::var(KAKOUNE_CLIENT).ok()),
        ),
    };

    let ctx = session.map(|session| Context::new(session, client.as_deref()));

    match (kamp.subcommand, ctx) {
        (Version(_), _) => cmd::version().map(Some),
        (Init(opt), _) => cmd::init(opt.export, opt.alias).map(Some),
        (Attach(opt), Some(ctx)) => cmd::attach(&ctx, opt.buffer).map(|_| None),
        (Edit(opt), Some(ctx)) => cmd::edit(&ctx, opt.files).map(|_| None),
        (Edit(opt), None) => kak::proxy(opt.files).map(|_| None),
        (Send(opt), Some(ctx)) => cmd::send(
            &ctx,
            join_command(opt.command, opt.remainder),
            to_csv_buffers_or_asterisk(opt.buffers),
        )
        .map(|_| None),
        (List(opt), ctx) if opt.all => cmd::list_all(ctx).map(Some),
        (List(_), Some(ctx)) => cmd::list(&ctx).map(Some),
        (Kill(opt), Some(ctx)) => cmd::kill(&ctx, opt.exit_status).map(|_| None),
        (Get(opt), Some(ctx)) => {
            use argv::GetSubCommand::*;
            let buffer = to_csv_buffers_or_asterisk(opt.buffers);
            let res = match opt.subcommand {
                Val(o) => ctx.query_val(&o.name, opt.raw, buffer),
                Opt(o) => ctx.query_opt(&o.name, opt.raw, buffer),
                Reg(o) => ctx.query_reg(&o.name, opt.raw, buffer),
                Shell(o) => ctx.query_sh(&o.name, opt.raw, buffer),
            };
            res.map(Some)
        }
        (Cat(opt), Some(ctx)) => cmd::cat(&ctx, to_csv_buffers_or_asterisk(opt.buffers)).map(Some),
        (Ctx(_), Some(ctx)) => cmd::ctx(&ctx).map(Some),
        _ => Err(Error::InvalidContext("session is required")),
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
    fn test_join_command() {
        assert_eq!(join_command("a".into(), vec![]), "a".to_owned());
        assert_eq!(
            join_command("a".into(), vec!["b".into(), "c".into()]),
            "a b c".to_owned()
        );
    }
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
