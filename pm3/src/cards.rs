//! Local aliases for card UIDs. Identifiers are not agency/account lookups.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

/// Only accept known message shapes. Unknown event data is never treated as UID.
/// ISORead has two prefix bytes; the documented DESFireRead has four.
pub fn uid_from_event(kind: u8, payload: &[u8]) -> Option<&[u8]> {
    match kind {
        0xb1 if matches!(payload.len(), 6 | 9 | 12) => Some(&payload[2..]),
        0xb9 if payload.len() == 11 => Some(&payload[4..]),
        _ => None,
    }
}

fn fingerprint(salt: &[u8], uid: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"pm3-card-alias-v1\0");
    hash.update(salt);
    hash.update(uid);
    format!("{:x}", hash.finalize())
}

fn event_fingerprint(salt: &[u8], kind: u8, payload: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"pm3-card-event-v1\0");
    hash.update(salt);
    hash.update([kind]);
    hash.update(payload);
    format!("{:x}", hash.finalize())
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentityMethod {
    #[default]
    Uid,
    ReaderEvent,
}

#[derive(Serialize, Deserialize)]
pub struct Card {
    fingerprint: String,
    pub label: String,
    pub taps: u64,
    #[serde(default)]
    pub method: IdentityMethod,
}

#[derive(Serialize, Deserialize)]
struct Database {
    version: u8,
    salt: Vec<u8>,
    cards: Vec<Card>,
}

pub struct Registry {
    path: PathBuf,
    database: Database,
}

impl Registry {
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let database = if path.exists() {
            Self::read(&path)?
        } else {
            let mut salt = vec![0; 32];
            File::open("/dev/urandom")?.read_exact(&mut salt)?;
            Database {
                version: 1,
                salt,
                cards: vec![],
            }
        };
        let registry = Self { path, database };
        if !registry.path.exists() {
            registry.save()?;
        }
        Ok(registry)
    }

    fn read(path: &Path) -> io::Result<Database> {
        let metadata = fs::metadata(path)?;
        if metadata.len() > 256 * 1024 {
            return Err(io::Error::other("card registry is too large"));
        }
        let database: Database = serde_json::from_slice(&fs::read(path)?)?;
        if database.version != 1 || database.salt.len() != 32 || database.cards.len() > 256 {
            return Err(io::Error::other("unsupported or invalid card registry"));
        }
        for (i, card) in database.cards.iter().enumerate() {
            if !valid_label(&card.label)
                || card.fingerprint.len() != 64
                || !card
                    .fingerprint
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
                || database.cards[..i]
                    .iter()
                    .any(|other| other.fingerprint == card.fingerprint || other.label == card.label)
            {
                return Err(io::Error::other("invalid or duplicate card registry entry"));
            }
        }
        Ok(database)
    }

    fn save(&self) -> io::Result<()> {
        let temporary = self
            .path
            .with_extension(format!("json.{}.tmp", std::process::id()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        let result = (|| {
            file.write_all(&serde_json::to_vec_pretty(&self.database)?)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            fs::rename(&temporary, &self.path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    pub fn recognize(&mut self, uid: &[u8]) -> io::Result<(String, String, bool)> {
        // Pick up labels edited by the CLI before applying this scan. Raw UIDs
        // are only hashed in memory; neither the file nor logs contain them.
        self.database = Self::read(&self.path)?;
        let token = fingerprint(&self.database.salt, uid);
        self.record_token(token, IdentityMethod::Uid)
    }

    /// Experimental equality of entire reader messages, not a decoded card UID.
    pub fn recognize_unhandled(
        &mut self,
        kind: u8,
        payload: &[u8],
    ) -> io::Result<(String, String, bool)> {
        if kind != 0xbe || !(4..=512).contains(&payload.len()) {
            return Err(io::Error::other("unsupported experimental card event"));
        }
        self.database = Self::read(&self.path)?;
        let token = event_fingerprint(&self.database.salt, kind, payload);
        self.record_token(token, IdentityMethod::ReaderEvent)
    }

    fn record_token(
        &mut self,
        token: String,
        method: IdentityMethod,
    ) -> io::Result<(String, String, bool)> {
        let existing = self
            .database
            .cards
            .iter()
            .position(|card| card.fingerprint == token);
        let index = if let Some(index) = existing {
            index
        } else {
            if self.database.cards.len() == 256 {
                return Err(io::Error::other("card registry is full"));
            }
            let mut number = self.database.cards.len() + 1;
            while self
                .database
                .cards
                .iter()
                .any(|card| card.label == format!("Card {number}"))
            {
                number += 1;
            }
            self.database.cards.push(Card {
                fingerprint: token.clone(),
                label: format!("Card {number}"),
                taps: 0,
                method,
            });
            self.database.cards.len() - 1
        };
        let card = &mut self.database.cards[index];
        card.taps = card.taps.saturating_add(1);
        let label = card.label.clone();
        self.save()?;
        Ok((label, token, existing.is_none()))
    }

    pub fn rename(&mut self, old: &str, new: &str) -> io::Result<()> {
        if !valid_label(new) {
            return Err(io::Error::other(
                "label must contain 1-32 printable characters",
            ));
        }
        self.database = Self::read(&self.path)?;
        if self
            .database
            .cards
            .iter()
            .any(|card| card.label == new && card.label != old)
        {
            return Err(io::Error::other("that card label already exists"));
        }
        let card = self
            .database
            .cards
            .iter_mut()
            .find(|card| card.label == old)
            .ok_or_else(|| io::Error::other("card label not found"))?;
        card.label = new.into();
        self.save()
    }

    pub fn cards(&self) -> &[Card] {
        &self.database.cards
    }
}

fn valid_label(label: &str) -> bool {
    !label.trim().is_empty() && label.chars().count() <= 32 && !label.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experimental_events_are_private_separate_and_repeatable() {
        let path = std::env::temp_dir().join(format!("pm3-event-test-{}.json", std::process::id()));
        let payload = b"synthetic-private-event";
        let mut registry = Registry::open(&path).unwrap();
        let first = registry.recognize_unhandled(0xbe, payload).unwrap();
        assert!(first.2);
        let repeat = Registry::open(&path)
            .unwrap()
            .recognize_unhandled(0xbe, payload)
            .unwrap();
        assert_eq!(first.1, repeat.1);
        assert!(!repeat.2);
        assert_ne!(first.1, registry.recognize(payload).unwrap().1);
        assert_ne!(
            first.1,
            registry
                .recognize_unhandled(0xbe, b"different-event")
                .unwrap()
                .1
        );
        assert!(registry.recognize_unhandled(0xb1, payload).is_err());
        assert!(registry.recognize_unhandled(0xbe, &[]).is_err());
        let content = fs::read_to_string(&path).unwrap();
        assert!(!content.contains("synthetic-private-event"));
        assert!(matches!(
            Registry::open(&path).unwrap().cards()[0].method,
            IdentityMethod::ReaderEvent
        ));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn same_uid_matches_across_iso_and_desfire_events() {
        let iso = [0x44, 0, 4, 1, 2, 3, 4, 5, 6];
        let desfire = [0x0c, 0, 0, 0, 4, 1, 2, 3, 4, 5, 6];
        assert_eq!(uid_from_event(0xb1, &iso), uid_from_event(0xb9, &desfire));
        assert_eq!(
            fingerprint(&[42; 32], &iso[2..]),
            fingerprint(&[42; 32], &desfire[4..])
        );
        assert!(uid_from_event(0xbe, &iso).is_none());
        assert!(uid_from_event(0xb1, &[0; 5]).is_none());
    }

    #[test]
    fn private_registry_keeps_labels_and_never_saves_raw_uid() {
        let path = std::env::temp_dir().join(format!("pm3-cards-test-{}.json", std::process::id()));
        let mut registry = Registry::open(&path).unwrap();
        let uid = [0xde, 0xad, 0xbe, 0xef, 0x11, 0x22, 0x33];
        assert_eq!(registry.recognize(&uid).unwrap().0, "Card 1");
        registry.rename("Card 1", "Test card").unwrap();
        assert_eq!(
            Registry::open(&path).unwrap().recognize(&uid).unwrap().0,
            "Test card"
        );
        assert_eq!(registry.recognize(&[1, 2, 3, 4]).unwrap().0, "Card 2");
        let content = fs::read_to_string(&path).unwrap();
        assert!(!content.contains("deadbeef112233"));
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::remove_file(path).unwrap();
    }
}
