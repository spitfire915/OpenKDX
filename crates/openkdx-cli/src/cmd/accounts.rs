use anyhow::anyhow;
use openkdx_client::accounts::{format_privileges, new_account, parse_privileges};
use openkdx_proto::msg::accounts::AccountClassInfo;
use openkdx_proto::msg::misc::{FolderAccess, FolderAccessRule};
use serde_json::json;

use super::Ctx;
use crate::args::{AccountsCmd, ClassesCmd, FaCmd};
use crate::out::{lossy, time_string};

pub async fn accounts(ctx: &mut Ctx, cmd: AccountsCmd) -> anyhow::Result<()> {
    let out = ctx.out;
    let session = ctx.session().await?.clone();
    match cmd {
        AccountsCmd::List => {
            let rows: Vec<Vec<serde_json::Value>> = session
                .accounts()
                .await?
                .iter()
                .map(|a| {
                    vec![
                        json!(lossy(&a.login)),
                        json!(lossy(&a.name)),
                        json!(a.class_index),
                        json!(time_string(a.created)),
                        json!(time_string(a.last_login)),
                    ]
                })
                .collect();
            out.table(
                "account",
                &["login", "name", "class", "created", "last_login"],
                &rows,
            );
        }
        AccountsCmd::Show { account: login } => {
            let a = session.account(login.as_bytes()).await?;
            out.record(
                "account",
                &[
                    ("login", json!(lossy(&a.login))),
                    ("name", json!(lossy(&a.name))),
                    ("class", json!(a.class_index)),
                    ("has_password", json!(a.has_password())),
                    ("privileges", json!(format_privileges(&a.privileges))),
                    (
                        "privilege_mask",
                        json!(format_privileges(&a.privilege_mask)),
                    ),
                    ("created", json!(time_string(a.created))),
                    ("modified", json!(time_string(a.modified))),
                    ("last_login", json!(time_string(a.last_login))),
                    ("editor", json!(lossy(&a.editor))),
                ],
            );
        }
        AccountsCmd::Create {
            account: login,
            new_password,
            name,
            class,
            privileges,
        } => {
            let privs = parse_privileges(&privileges)?;
            let account = new_account(
                login.as_bytes(),
                name.as_bytes(),
                Some(new_password.as_bytes()),
                class,
                privs,
                privs,
            );
            session.create_account(account).await?;
            out.note(format!("created {login}"));
        }
        AccountsCmd::Set {
            account: login,
            new_password,
            name,
            class,
            privileges,
        } => {
            let mut a = session.account(login.as_bytes()).await?;
            a.key = match new_password {
                Some(p) => openkdx_proto::AccountKey::from_password(p.as_bytes()).0,
                None => [0; 32],
            };
            if let Some(n) = name {
                a.name = n.into_bytes();
            }
            if let Some(c) = class {
                a.class_index = c;
            }
            if let Some(p) = privileges {
                let privs = parse_privileges(&p)?;
                a.privileges = privs;
                a.privilege_mask = privs;
            }
            session.set_account(a).await?;
        }
        AccountsCmd::Delete { account: login } => {
            session.delete_account(login.as_bytes()).await?;
        }
        AccountsCmd::Passwd { new_password } => {
            session.change_own_password(new_password.as_bytes()).await?;
        }
    }
    Ok(())
}

pub async fn classes(ctx: &mut Ctx, cmd: ClassesCmd) -> anyhow::Result<()> {
    let out = ctx.out;
    let session = ctx.session().await?.clone();
    match cmd {
        ClassesCmd::List => {
            let rows: Vec<Vec<serde_json::Value>> = session
                .classes()
                .await?
                .iter()
                .map(|c| {
                    vec![
                        json!(c.class_index),
                        json!(lossy(&c.name)),
                        json!(c.speed_limit),
                        json!(format!(
                            "{:02x}{:02x}{:02x}",
                            c.color.r, c.color.g, c.color.b
                        )),
                        json!(format_privileges(&c.privileges)),
                    ]
                })
                .collect();
            out.table(
                "class",
                &["index", "name", "speed_limit", "color", "privileges"],
                &rows,
            );
        }
        ClassesCmd::Show { index } => {
            let c = session.class(index).await?;
            out.record(
                "class",
                &[
                    ("index", json!(c.class_index)),
                    ("name", json!(lossy(&c.name))),
                    ("speed_limit", json!(c.speed_limit)),
                    ("flood_protection", json!(c.flood_protection)),
                    ("privileges", json!(format_privileges(&c.privileges))),
                    ("editor", json!(lossy(&c.editor_login))),
                ],
            );
        }
        ClassesCmd::Set {
            index,
            name,
            privileges,
            speed_limit,
            flood,
        } => {
            let mut c = session.class(index).await?;
            if let Some(n) = name {
                c.name = n.into_bytes();
            }
            if let Some(p) = privileges {
                c.privileges = parse_privileges(&p)?;
            }
            if let Some(s) = speed_limit {
                c.speed_limit = s;
            }
            if let Some(f) = flood {
                c.flood_protection = match f.as_str() {
                    "relaxed" => AccountClassInfo::FLOOD_RELAXED,
                    "average" => AccountClassInfo::FLOOD_AVERAGE,
                    "strict" => AccountClassInfo::FLOOD_STRICT,
                    _ => return Err(anyhow!("flood: relaxed, average, or strict")),
                };
            }
            session.set_class(c).await?;
        }
    }
    Ok(())
}

fn icon_code(name: &str) -> anyhow::Result<u32> {
    Ok(match name {
        "normal" => FolderAccess::ICON_NORMAL,
        "uploads" => FolderAccess::ICON_UPLOADS,
        "dropbox" => FolderAccess::ICON_DROPBOX,
        "pictures" => FolderAccess::ICON_PICTURES,
        "audio" => FolderAccess::ICON_AUDIO,
        _ => {
            return Err(anyhow!(
                "icon: normal, uploads, dropbox, pictures, or audio"
            ));
        }
    })
}

fn icon_name(code: u32) -> String {
    match code {
        FolderAccess::ICON_NORMAL => "normal".into(),
        FolderAccess::ICON_UPLOADS => "uploads".into(),
        FolderAccess::ICON_DROPBOX => "dropbox".into(),
        FolderAccess::ICON_PICTURES => "pictures".into(),
        FolderAccess::ICON_AUDIO => "audio".into(),
        other => format!("{other:#x}"),
    }
}

pub async fn folder_access(ctx: &mut Ctx, cmd: FaCmd) -> anyhow::Result<()> {
    let out = ctx.out;
    let session = ctx.session().await?.clone();
    match cmd {
        FaCmd::List => {
            let rows: Vec<Vec<serde_json::Value>> = session
                .folder_access_list()
                .await?
                .iter()
                .map(|i| {
                    vec![
                        json!(lossy(&i.name)),
                        json!(icon_name(i.icon)),
                        json!(format!("{:#x}", i.flags)),
                        json!(lossy(&i.description)),
                    ]
                })
                .collect();
            out.table(
                "folder-access",
                &["name", "icon", "flags", "description"],
                &rows,
            );
        }
        FaCmd::Show { name } => {
            let i = session.folder_access(name.as_bytes()).await?;
            let rules: Vec<String> = i
                .class_rules
                .iter()
                .enumerate()
                .filter(|(_, r)| r.permission_bits != 0)
                .map(|(n, r)| format!("{n}:{:#x}", r.permission_bits))
                .collect();
            out.record(
                "folder-access",
                &[
                    ("name", json!(lossy(&i.name))),
                    ("icon", json!(icon_name(i.icon))),
                    ("flags", json!(format!("{:#x}", i.flags))),
                    ("owner", json!(lossy(&i.owner))),
                    ("description", json!(lossy(&i.description))),
                    ("rule", json!(format!("{:#x}", i.rule.permission_bits))),
                    ("class_rules", json!(rules)),
                ],
            );
        }
        FaCmd::Create {
            name,
            icon,
            owner,
            description,
        } => {
            session
                .create_folder_access(FolderAccess {
                    ignored_00: [0; 4],
                    flags: 0,
                    icon: icon_code(&icon)?,
                    class_rules: [FolderAccessRule::default(); 32],
                    rule: FolderAccessRule::default(),
                    ignored_90: [0; 29],
                    ignored_b0: 0,
                    ignored_b2: [0; 6],
                    name: name.into_bytes(),
                    owner: owner.into_bytes(),
                    description: description.into_bytes(),
                    current_name: Vec::new(),
                    padding: Vec::new(),
                })
                .await?;
        }
        FaCmd::Delete { name } => {
            session.delete_folder_access(name.as_bytes()).await?;
        }
    }
    Ok(())
}
