//! Accounts, classes, and folder access: 502–551 and 1100–1107.

use std::time::{SystemTime, UNIX_EPOCH};

use openkdx_proto::msg::accounts::{
    Account, AccountClassInfo, AccountClassList, AccountClassListEntry, AccountInfo, AccountList,
    AccountListEntry, ChangeOwnPassword, ChangeOwnPasswordDone, CreateAccount, DeleteAccount,
    GetAccount, GetAccountClass, GetAccountClassList, GetAccountList, Privilege, Privileges,
    SetAccount, SetAccountClass,
};
use openkdx_proto::msg::common::Time1900;
use openkdx_proto::msg::misc::{
    CreateFolderAccess, DeleteFolderAccess, FolderAccess, FolderAccessInfo, FolderAccessItem,
    FolderAccessList, GetFolderAccess, GetFolderAccessList, SetFolderAccess,
};
use openkdx_proto::{AccountKey, Message, Payload};

use crate::session::Session;
use crate::{ClientError, Result};

fn now() -> Time1900 {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Time1900::from_unix_ms(ms)
}

/// An account record for 507/511. `password: None` keeps the stored key
/// (all-zero key on the wire).
pub fn new_account(
    login: &[u8],
    name: &[u8],
    password: Option<&[u8]>,
    class_index: u8,
    privileges: Privileges,
    privilege_mask: Privileges,
) -> Account {
    let t = now();
    Account {
        privileges,
        privilege_mask,
        key: password
            .map(|p| AccountKey::from_password(p).0)
            .unwrap_or([0; 32]),
        created: t,
        modified: t,
        last_login: Time1900(0),
        class_index,
        login: login.to_vec(),
        name: name.to_vec(),
        ..Account::default()
    }
}

/// Records with a trailing pad go out padded to a multiple of 4: the
/// server checks the payload length (reference `folder-access.md`).
fn padded<T: Payload>(mut msg: T, set_padding: impl Fn(&mut T, Vec<u8>)) -> T {
    let len = msg.to_bytes().len();
    set_padding(&mut msg, vec![0; openkdx_proto::wire::pad4_len(len)]);
    msg
}

/// Privilege names in kebab-case: `can-login`.
pub fn privilege_slug(p: Privilege) -> String {
    let mut out = String::new();
    for (i, ch) in p.name().chars().enumerate() {
        if ch.is_uppercase() && i > 0 && !out.ends_with('-') {
            out.push('-');
        }
        if ch == ' ' || ch == '/' {
            if !out.ends_with('-') {
                out.push('-');
            }
            continue;
        }
        if ch.is_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        }
    }
    out.trim_matches('-').to_string()
}

/// `all`, `none`, or comma-separated slugs (case-insensitive, spaces ignored).
pub fn parse_privileges(spec: &str) -> Result<Privileges> {
    let spec = spec.trim();
    if spec.eq_ignore_ascii_case("all") {
        return Ok(!Privileges::NONE);
    }
    if spec.is_empty() || spec.eq_ignore_ascii_case("none") {
        return Ok(Privileges::NONE);
    }
    let mut out = Privileges::NONE;
    for word in spec.split(',') {
        let word = word.trim().to_ascii_lowercase();
        let found = Privilege::ALL
            .iter()
            .copied()
            .find(|p| privilege_slug(*p) == word)
            .or_else(|| {
                word.strip_prefix("bit")
                    .and_then(|n| n.parse::<u8>().ok())
                    .and_then(Privilege::from_bit)
            });
        match found {
            Some(p) => out.set(p, true),
            None => return Err(ClientError::Protocol("unknown privilege name")),
        }
    }
    Ok(out)
}

pub fn format_privileges(p: &Privileges) -> String {
    let names: Vec<String> = p.set_bits().into_iter().map(privilege_slug).collect();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(",")
    }
}

impl Session {
    /// 512 → 513.
    pub async fn accounts(&self) -> Result<Vec<AccountListEntry>> {
        let list: AccountList = self.connection().request_as(&GetAccountList).await?;
        Ok(list.accounts)
    }

    /// 509 → 510.
    pub async fn account(&self, login: &[u8]) -> Result<Account> {
        let info: AccountInfo = self
            .connection()
            .request_as(&GetAccount {
                login: login.to_vec(),
            })
            .await?;
        Ok(info.account)
    }

    /// 507 → 1 Status 0x1fb.
    pub async fn create_account(&self, mut account: Account) -> Result<Message> {
        account.padding.clear();
        let msg = padded(CreateAccount { account }, |m, p| m.account.padding = p);
        self.connection().request_ok(&msg).await
    }

    /// 511 → 1 Status 0x1ff. On the wire `editor` names the account to
    /// change; this sets it to `login` (510 returns the last editor there,
    /// which would read as a rename). Use `rename_account` to rename.
    pub async fn set_account(&self, mut account: Account) -> Result<Message> {
        account.editor = account.login.clone();
        account.padding.clear();
        let msg = padded(SetAccount { account }, |m, p| m.account.padding = p);
        self.connection().request_ok(&msg).await
    }

    /// 511 with `editor` = the stored login and `login` = the new one.
    pub async fn rename_account(&self, current: &[u8], mut account: Account) -> Result<Message> {
        account.editor = current.to_vec();
        account.padding.clear();
        let msg = padded(SetAccount { account }, |m, p| m.account.padding = p);
        self.connection().request_ok(&msg).await
    }

    /// 508; every user gets 515 with the login.
    pub async fn delete_account(&self, login: &[u8]) -> Result<()> {
        let mut events = self.events();
        self.connection()
            .send(&DeleteAccount {
                login: login.to_vec(),
            })
            .await?;
        self.wait_for(&mut events, |m| match m {
            Message::AccountDeleted(d) if d.login == login => Some(Ok(())),
            Message::Status(s) if s.is_error() => Some(Err(ClientError::Server {
                code: s.code,
                name: openkdx_proto::msg::error_name(s.code),
                request: DeleteAccount::KIND.0,
            })),
            _ => None,
        })
        .await?
    }

    /// 550 → 551.
    pub async fn change_own_password(&self, new_password: &[u8]) -> Result<()> {
        self.connection()
            .request_as::<_, ChangeOwnPasswordDone>(&ChangeOwnPassword {
                account_key: AccountKey::from_password(new_password).0,
            })
            .await
            .map(|_| ())
    }

    /// 505 → 506.
    pub async fn classes(&self) -> Result<Vec<AccountClassListEntry>> {
        let list: AccountClassList = self.connection().request_as(&GetAccountClassList).await?;
        Ok(list.classes)
    }

    /// 502 → 503.
    pub async fn class(&self, class_index: u8) -> Result<AccountClassInfo> {
        self.connection()
            .request_as(&GetAccountClass { class_index })
            .await
    }

    /// 504 → whatever the server answers (a Status or 503).
    pub async fn set_class(&self, mut class: AccountClassInfo) -> Result<Message> {
        class.padding.clear();
        let msg = padded(SetAccountClass { class }, |m, p| m.class.padding = p);
        self.connection().request_ok(&msg).await
    }

    /// 1106 (flags 0) → 1107.
    pub async fn folder_access_list(&self) -> Result<Vec<FolderAccessItem>> {
        let list: FolderAccessList = self
            .connection()
            .request_as(&GetFolderAccessList {
                rest: 0u32.to_be_bytes().to_vec(),
            })
            .await?;
        Ok(list.items)
    }

    /// 1103 → 1104.
    pub async fn folder_access(&self, name: &[u8]) -> Result<FolderAccess> {
        let info: FolderAccessInfo = self
            .connection()
            .request_as(&GetFolderAccess {
                name: name.to_vec(),
            })
            .await?;
        Ok(info.item)
    }

    /// 1100 → 1 Status 0x44c.
    pub async fn create_folder_access(&self, mut item: FolderAccess) -> Result<Message> {
        item.padding.clear();
        let msg = padded(CreateFolderAccess { item }, |m, p| m.item.padding = p);
        self.connection().request_ok(&msg).await
    }

    /// 1105 → Status. `current_name` names the item to change.
    pub async fn set_folder_access(&self, mut item: FolderAccess) -> Result<Message> {
        if item.current_name.is_empty() {
            item.current_name = item.name.clone();
        }
        item.padding.clear();
        let msg = padded(SetFolderAccess { item }, |m, p| m.item.padding = p);
        self.connection().request_ok(&msg).await
    }

    /// 1101; every user gets 1102.
    pub async fn delete_folder_access(&self, name: &[u8]) -> Result<()> {
        let mut events = self.events();
        self.connection()
            .send(&DeleteFolderAccess {
                name: name.to_vec(),
            })
            .await?;
        self.wait_for(&mut events, |m| match m {
            Message::FolderAccessDeleted(d) if d.name == name => Some(Ok(())),
            Message::Status(s) if s.is_error() => Some(Err(ClientError::Server {
                code: s.code,
                name: openkdx_proto::msg::error_name(s.code),
                request: DeleteFolderAccess::KIND.0,
            })),
            _ => None,
        })
        .await?
    }
}
