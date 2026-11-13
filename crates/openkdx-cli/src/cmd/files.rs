use openkdx_client::path::{join, parse_path, path_string};
use openkdx_client::transfer::{Progress, Transfer};
use openkdx_proto::msg::files::FileListEntry;
use serde_json::json;

use super::Ctx;
use crate::args::FilesCmd;
use crate::out::{lossy, time_string};

/// One line at the end: `N bytes`; with `--json`, nothing until the record.
struct ProgressLine {
    out: crate::out::Out,
    first: Option<Progress>,
    last: Option<Progress>,
}

impl ProgressLine {
    fn new(out: crate::out::Out) -> Self {
        ProgressLine {
            out,
            first: None,
            last: None,
        }
    }

    fn update(&mut self, p: Progress) {
        if self.first.is_none() {
            self.first = Some(p);
        }
        self.last = Some(p);
        if p.done == p.total {
            self.out.note(format!("{} bytes", p.total));
        }
    }
}

fn access_tag(entry: &FileListEntry) -> &'static str {
    match entry.access_item {
        0x2c => "[DB]",
        0x2b => "[UL]",
        0 => "",
        _ => "[FA]",
    }
}

pub async fn files(ctx: &mut Ctx, cmd: FilesCmd) -> anyhow::Result<()> {
    let out = ctx.out;
    let session = ctx.session().await?.clone();
    match cmd {
        FilesCmd::Ls { path, long } => {
            let entries = session.list_files(&parse_path(&path)).await?;
            let rows: Vec<Vec<serde_json::Value>> = entries
                .iter()
                .map(|e| {
                    let name = if e.is_folder() {
                        format!("{}/", lossy(&e.name))
                    } else {
                        lossy(&e.name)
                    };
                    if long || out.json {
                        vec![
                            json!(name),
                            json!(if e.is_folder() {
                                json!(null)
                            } else {
                                json!(e.size)
                            }),
                            json!(time_string(e.modified)),
                            json!(lossy(&e.kind)),
                            json!(access_tag(e)),
                        ]
                    } else {
                        vec![json!(name), json!(access_tag(e))]
                    }
                })
                .collect();
            if long || out.json {
                out.table(
                    "file",
                    &["name", "size", "modified", "type", "access"],
                    &rows,
                );
            } else {
                out.table("file", &["name", "access"], &rows);
            }
        }
        FilesCmd::Info { path } => {
            let info = session.file_info(&parse_path(&path)).await?;
            out.record(
                "file-info",
                &[
                    ("name", json!(lossy(&info.name))),
                    ("size", json!(info.size)),
                    ("created", json!(time_string(info.created))),
                    ("modified", json!(time_string(info.modified))),
                    ("type", json!(lossy(&info.kind))),
                ],
            );
        }
        FilesCmd::Mkdir {
            parent,
            name,
            access,
            owner,
        } => {
            session
                .create_folder(
                    &parse_path(&parent),
                    name.as_bytes(),
                    access.as_bytes(),
                    owner.as_bytes(),
                )
                .await?;
            out.note(format!(
                "created {}",
                path_string(&join(&parse_path(&parent), name.as_bytes()))
            ));
        }
        FilesCmd::Trash { paths } => {
            for p in paths {
                let mut path = parse_path(&p);
                let Some(last) = path.names.pop() else {
                    anyhow::bail!("cannot trash the base folder");
                };
                session.move_to_trash(&path, &[&last.name]).await?;
            }
        }
        FilesCmd::EmptyTrash => {
            session.empty_trash().await?;
        }
        FilesCmd::Catalog => {
            let items = session.generate_catalog().await?;
            out.record("catalog", &[("items", json!(items))]);
        }
        FilesCmd::Search { text } => {
            let rows: Vec<Vec<serde_json::Value>> = session
                .search(text.as_bytes())
                .await?
                .iter()
                .map(|e| {
                    vec![
                        json!(lossy(&e.name)),
                        json!(e.size),
                        json!(path_string(&e.path)),
                    ]
                })
                .collect();
            out.table("search", &["name", "size", "path"], &rows);
        }
        FilesCmd::Get { path, to, resume } => {
            let remote = parse_path(&path);
            let mut t = Transfer::open(&session).await?;
            let mut progress = ProgressLine::new(out);
            let dest = match to {
                Some(p) if p.is_dir() => {
                    t.download_to_dir(&remote, &p, resume, |p| progress.update(p))
                        .await?
                }
                Some(p) => {
                    t.download(&remote, &p, resume, |p| progress.update(p))
                        .await?;
                    p
                }
                None => {
                    t.download_to_dir(&remote, std::path::Path::new("."), resume, |p| {
                        progress.update(p)
                    })
                    .await?
                }
            };
            t.close().await;
            out.record(
                "downloaded",
                &[
                    ("path", json!(path_string(&remote))),
                    ("to", json!(dest.to_string_lossy())),
                    ("bytes", json!(progress.last.map(|p| p.total))),
                ],
            );
        }
        FilesCmd::Put {
            local,
            folder,
            name,
            resume,
            stop_after,
        } => {
            let name = name.unwrap_or_else(|| {
                local
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
            let mut t = Transfer::open(&session).await?;
            let mut progress = ProgressLine::new(out);
            t.upload(
                &local,
                &parse_path(&folder),
                name.as_bytes(),
                resume,
                |p| progress.update(p),
                stop_after,
            )
            .await?;
            t.close().await;
            out.record(
                "uploaded",
                &[
                    (
                        "to",
                        json!(path_string(&join(&parse_path(&folder), name.as_bytes()))),
                    ),
                    ("bytes", json!(progress.last.map(|p| p.total))),
                    ("from", json!(progress.first.map(|p| p.done))),
                ],
            );
        }
    }
    Ok(())
}
