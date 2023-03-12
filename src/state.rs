use std::io;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Debug)]
pub struct State {
    inner: Arc<StateInner>,
}

impl State {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(StateInner {
                dumps: Dumps {
                    next_id: AtomicU64::new(0),
                },
                // 500 MB
                max_size: 500_000_00,
            }),
        }
    }
}

impl Deref for State {
    type Target = StateInner;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
pub struct StateInner {
    pub dumps: Dumps,
    pub max_size: u64,
}

#[derive(Debug)]
pub struct Dumps {
    next_id: AtomicU64,
}

impl Dumps {
    pub async fn insert(&self) -> io::Result<Dump> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let path = format!("./app/partial/{}", id);

        Dump::new(path).await
    }

    pub async fn get(&self, id: &str) -> io::Result<File> {
        let path = format!("./app/dumps/{}", id);
        File::open(path).await
    }
}

#[derive(Debug)]
pub struct Dump {
    path: String,
    hasher: Sha256,
    file: File,
}

impl Dump {
    pub async fn new(path: String) -> io::Result<Self> {
        let file = File::create(&path).await?;

        Ok(Self {
            path,
            file,
            hasher: Sha256::new(),
        })
    }

    pub async fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        self.hasher.update(buf);
        self.file.write_all(buf).await
    }

    pub async fn finish(self) -> io::Result<String> {
        let hash = hex::encode(self.hasher.finalize());

        let path = format!("./app/dumps/{}", hash);

        tokio::fs::rename(self.path, path).await?;
        Ok(hash)
    }

    pub async fn abort(self) -> io::Result<()> {
        drop(self.file);
        tokio::fs::remove_file(self.path).await
    }
}
