use crate::util::hash_serde;
use blake3::{Hash, Hasher};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Digest {
    version: u32,
    pub size: u64,
    pub downloaded: u64,

    pub file_name: String,
    pub content_type: String,

    #[serde(with = "hash_serde")]
    hash: Hash,

    #[serde(skip_serializing, default = "default_root")]
    root: PathBuf,
}

impl Digest {
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Digest, DigestError> {
        let path = path.as_ref();
        let root = path.parent().ok_or(DigestError::FileNotFound)?.to_owned();

        let text = std::fs::read_to_string(path)?;
        let mut digest: Digest = serde_json::from_str(&text)?;

        digest.root = root;

        Ok(digest)
    }

    pub fn for_path<P: AsRef<Path>>(path: P) -> Result<Digest, DigestError> {
        let path = path.as_ref();
        let file_name = path
            .file_name()
            .ok_or(DigestError::FileNotFound)?
            .to_str()
            .ok_or(DigestError::InvalidFileName)?;

        let digest_path = format!("{}.digest", file_name);

        Self::from_file(
            path.parent()
                .ok_or(DigestError::FileNotFound)?
                .join(digest_path),
        )
    }

    pub fn write<P: AsRef<Path>>(&self, root: P) -> Result<(), DigestError> {
        let root = root.as_ref();

        let digest_filename = format!("{}.digest", self.file_name);
        let digest_path = root.join(digest_filename);

        let file = fs::File::create(digest_path)?;
        serde_json::to_writer_pretty(file, &self)?;

        Ok(())
    }

    pub fn new<P>(path: P, content_type: &str, hash: Hash) -> Self
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let m = fs::metadata(path).unwrap();
        let file_name = path.file_name().unwrap().to_str().unwrap().to_owned();

        let root = path.parent().unwrap().to_owned();

        Digest {
            version: 1,
            size: m.len(),
            downloaded: m.len(),
            file_name,
            content_type: content_type.to_owned(),
            hash,
            root,
        }
    }

    pub fn verify(&self) -> Result<(), DigestError> {
        let mut hasher = Hasher::new();
        let mut file = io::BufReader::new(fs::File::open(self.get_file_path())?);
        io::copy(&mut file, &mut hasher)?;

        let hash = hasher.finalize();
        if hash != self.hash {
            Err(DigestError::VerifyError)
        } else {
            Ok(())
        }
    }

    pub fn serve(&self) -> impl actix_web::Responder {
        use crate::util::named_file::NamedFile;

        NamedFile::open(self.get_file_path())
            .unwrap()
           // .set_content_type(self.content_type.parse().unwrap())
    }

    pub fn get_file_path(&self) -> PathBuf {
        self.root.join(&self.file_name)
    }

    fn get_digest_path(&self) -> PathBuf {
        self.root.join(format!("{}.digest", self.file_name))
    }
}

fn default_root() -> PathBuf {
    PathBuf::new()
}

#[derive(Error, Debug)]
pub enum DigestError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error("File to create digest from does not exist")]
    FileNotFound,

    #[error("File name to create digest from is not valid")]
    InvalidFileName,

    #[error("Incorrect hash in digest")]
    VerifyError,
}
