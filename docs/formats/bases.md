# `Bases/` conventions

**Status:** read. Nothing to write: the server reads the directory as it is.

A KDX server shares one or more *bases*, each a directory under `Bases/`.
A fresh Server 1.620 creates `Bases/Default/` with two folders
`[O lab first-run]`:

```
Bases/Default/Admin Dropbox [DB]
Bases/Default/Uploads [UL]
```

A folder name that ends in a space, `[`, a tag, and `]` binds the folder to a
Folder Access item (reference `messages/misc.md`, 1100–1107):

| Suffix | Binds | Notes |
|---|---|---|
| ` [UL]` | the built-in Uploads item | icon 0x2b on the wire |
| ` [DB]` | the built-in Drop Box item | icon 0x2c; the owner's login is in the name when the item's flag bit 1 (Owner Login In Folder Name) is set |
| ` [<item>]` | the Folder Access item named `<item>` | created by Folder Access → New; `[O folder-access]` |

The display name the client shows is everything before the ` [`. A `[`
without the leading space, or an empty name before it, is just part of the
name `[I]`.

`openkdx_legacy::bases::{parse_base_name, format_base_name}` implement this;
`format_base_name(&parse_base_name(n)) == n` for every name.
