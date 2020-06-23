use crate::hash_serde;
use blake3::{Hash, Hasher};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Manifest {
    version: u32,
    path: PathBuf,
    content_type: String,

    #[serde(with = "hash_serde")]
    hash: Hash,
}

impl Manifest {
    pub fn from_file<T: AsRef<Path>>(path: T) -> Result<Manifest, ManifestError> {
        let text = std::fs::read_to_string(path)?;
        let manifest = serde_json::from_str(&text)?;
        Ok(manifest)
    }

    pub fn new<P>(path: P, content_type: &str, hash: Hash) -> Self
    where
        P: AsRef<Path>,
    {
        Manifest {
            version: 1,
            path: path.as_ref().to_path_buf(),
            content_type: content_type.to_owned(),
            hash,
        }
    }

    pub fn verify(&self) -> Result<(), ManifestError> {
        let mut hasher = Hasher::new();
        let mut file = io::BufReader::new(fs::File::open(&self.path)?);
        io::copy(&mut file, &mut hasher)?;

        let hash = hasher.finalize();
        if hash != self.hash {
            Err(ManifestError::VerifyError)
        } else {
            Ok(())
        }
    }

    pub fn serve(&self) -> impl actix_web::Responder {
        use actix_files::NamedFile;

        NamedFile::open(&self.path)
            .unwrap()
            .set_content_type(self.content_type.parse().unwrap())
    }
}

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error("Incorrect hash in manifest")]
    VerifyError,
}
