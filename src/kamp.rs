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

pub(super) fn run() -> Result<()> {
    let kamp: Kampliment = argh::from_env();
    if kamp.version {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if let Some(subcommand) = kamp.subcommand {
        let (session, client) = match (kamp.session, kamp.client.filter(|s| !s.is_empty())) {
            (Some(s), client) => (Some(s), client),
            (None, client) => (
                std::env::var(KAKOUNE_SESSION).ok(),
                client.or_else(|| std::env::var(KAKOUNE_CLIENT).ok()),
            ),
        };
        match (subcommand, session.as_deref()) {
            (Init(opt), _) => {
                let res = cmd::init(opt.export, opt.alias)?;
                print!("{res}");
            }
            (Attach(opt), Some(session)) => {
                let ctx = Context::new(session, client.as_deref());
                return cmd::attach(ctx, opt.buffer);
            }
            (Edit(opt), Some(session)) => {
                let ctx = Context::new(session, client.as_deref());
                return cmd::edit(ctx, opt.files);
            }
            (Edit(opt), None) => {
                return kak::proxy(opt.files);
            }
            (Send(opt), Some(session)) => {
                if opt.command.is_empty() {
                    return Err(Error::CommandRequired);
                }
                let ctx = Context::new(session, client.as_deref());
                let (buffers, _) = to_csv_buffers_or_asterisk(opt.buffers);
                let res = ctx.send(opt.command.join(" "), buffers)?;
                print!("{res}");
            }
            (List(opt), _) if opt.all => {
                for session in cmd::list_all()? {
                    println!("{:#?}", session);
                }
            }
            (List(_), Some(session)) => {
                let ctx = Context::new(session, client.as_deref());
                let session = cmd::list_current(ctx)?;
                println!("{:#?}", session);
            }
            (Kill(opt), Some(session)) => {
                let ctx = Context::new(session, client.as_deref());
                return ctx.send_kill(opt.exit_status);
            }
            (Get(opt), Some(session)) => {
                use argv::GetSubCommand::*;
                use context::SplitType;
                let ctx = Context::new(session, client.as_deref());
                let res = match opt.subcommand {
                    Val(o) => {
                        let (buffers, more_than_one) = to_csv_buffers_or_asterisk(o.buffers);
                        let split_type =
                            SplitType::new(o.quote, o.split || o.split0, more_than_one);
                        ctx.query_val(o.name, split_type, buffers)
                            .map(|v| (v, o.split0))
                    }
                    Opt(o) => {
                        let (buffers, more_than_one) = to_csv_buffers_or_asterisk(o.buffers);
                        let split_type =
                            SplitType::new(o.quote, o.split || o.split0, more_than_one);
                        ctx.query_opt(o.name, split_type, buffers)
                            .map(|v| (v, o.split0))
                    }
                    Reg(o) => {
                        let split_type = SplitType::new(o.quote, o.split || o.split0, false);
                        ctx.query_reg(o.name, split_type).map(|v| (v, o.split0))
                    }
                    Shell(o) => {
                        if o.command.is_empty() {
                            Err(Error::CommandRequired)
                        } else {
                            let (buffers, _) = to_csv_buffers_or_asterisk(o.buffers);
                            ctx.query_sh(o.command.join(" "), SplitType::None(false), buffers)
                                .map(|v| (v, false))
                        }
                    }
                };
                let (items, split0) = res?;
                let split_char = if split0 { '\0' } else { '\n' };
                for item in items {
                    print!("{item}{split_char}");
                }
            }
            (Cat(opt), Some(session)) => {
                let ctx = Context::new(session, client.as_deref());
                let (buffers, _) = to_csv_buffers_or_asterisk(opt.buffers);
                let res = cmd::cat(ctx, buffers)?;
                print!("{res}");
            }
            (Ctx(_), Some(session)) => {
                println!("session: {session}");
                if let Some(client) = &client {
                    println!("client: {client}");
                }
            }
            _ => return Err(Error::InvalidContext("session is required")),
        }
    }

    Ok(())
}

fn to_csv_buffers_or_asterisk(buffers: Vec<String>) -> (Option<String>, bool) {
    if buffers.is_empty() {
        return (None, false);
    }
    if buffers[0] == "*" {
        return (buffers.into_iter().rev().last(), true);
    }
    let mut count = 0;
    let mut res =
        buffers
            .into_iter()
            .filter(|s| s != "*")
            .fold(String::from('\''), |mut buf, next| {
                count += 1;
                buf.push_str(&next);
                buf.push(',');
                buf
            });
    res.pop(); // pops last ','
    res.push('\'');
    (Some(res), count > 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_to_csv_buffers_or_asterisk() {
        assert_eq!(to_csv_buffers_or_asterisk(vec![]), (None, false));
        assert_eq!(
            to_csv_buffers_or_asterisk(vec!["*".into()]),
            (Some("*".into()), true)
        );
        assert_eq!(
            to_csv_buffers_or_asterisk(vec!["*".into(), "a".into()]),
            (Some("*".into()), true)
        );
        assert_eq!(
            to_csv_buffers_or_asterisk(vec!["a".into(), "*".into()]),
            (Some("'a'".into()), false)
        );
        assert_eq!(
            to_csv_buffers_or_asterisk(vec!["a".into()]),
            (Some("'a'".into()), false)
        );
        assert_eq!(
            to_csv_buffers_or_asterisk(vec!["a".into(), "b".into()]),
            (Some("'a,b'".into()), true)
        );
    }
}
