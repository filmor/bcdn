use std::fs::{File, Metadata};
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use actix_http::body::SizedStream;
use actix_web::dev::BodyEncoding;
use actix_web::http::header::{
    self, Charset, ContentDisposition, DispositionParam, DispositionType, ExtendedValue,
};
use actix_web::http::{ContentEncoding, StatusCode};
use actix_web::{Error, HttpMessage, HttpRequest, HttpResponse, Responder};
use futures_util::future::{ready, Ready};

use super::range::HttpRange;
use super::chunked_read_file::ChunkedReadFile;

/// A file with an associated name.
#[derive(Debug)]
pub struct NamedFile {
    path: PathBuf,
    file: File,
    modified: Option<SystemTime>,
    pub(crate) md: Metadata,
    pub(crate) status_code: StatusCode,
    pub(crate) content_type: String,
    pub(crate) encoding: Option<ContentEncoding>,
}

impl NamedFile {
    /// Creates an instance from a previously opened file.
    ///
    /// The given `path` need not exist and is only used to determine the `ContentType` and
    /// `ContentDisposition` headers.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use actix_files::NamedFile;
    /// use std::io::{self, Write};
    /// use std::env;
    /// use std::fs::File;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut file = File::create("foo.txt")?;
    ///     file.write_all(b"Hello, world!")?;
    ///     let named_file = NamedFile::from_file(file, "bar.txt")?;
    ///     # std::fs::remove_file("foo.txt");
    ///     Ok(())
    /// }
    /// ```
    pub fn from_file<P: AsRef<Path>>(file: File, path: P) -> io::Result<NamedFile> {
        let path = path.as_ref().to_path_buf();

        // Get the name of the file and use it to construct default Content-Type
        // and Content-Disposition values
        let (content_type, content_disposition) = {
            let filename = match path.file_name() {
                Some(name) => name.to_string_lossy(),
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Provided path has no filename",
                    ));
                }
            };

            let disposition = DispositionType::Attachment;
            let mut parameters =
                vec![DispositionParam::Filename(String::from(filename.as_ref()))];
            if !filename.is_ascii() {
                parameters.push(DispositionParam::FilenameExt(ExtendedValue {
                    charset: Charset::Ext(String::from("UTF-8")),
                    language_tag: None,
                    value: filename.into_owned().into_bytes(),
                }))
            }
            let cd = ContentDisposition {
                disposition,
                parameters,
            };
            ("application/octet-stream".to_owned(), cd)
        };

        let md = file.metadata()?;
        let modified = md.modified().ok();
        let encoding = None;
        Ok(NamedFile {
            path,
            file,
            content_type,
            md,
            modified,
            encoding,
            status_code: StatusCode::OK,
        })
    }

    /// Attempts to open a file in read-only mode.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use actix_files::NamedFile;
    ///
    /// let file = NamedFile::open("foo.txt");
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<NamedFile> {
        Self::from_file(File::open(&path)?, path)
    }

    /// Returns reference to the underlying `File` object.
    #[inline]
    pub fn file(&self) -> &File {
        &self.file
    }

    /// Retrieve the path of this file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::io;
    /// use actix_files::NamedFile;
    ///
    /// # fn path() -> io::Result<()> {
    /// let file = NamedFile::open("test.txt")?;
    /// assert_eq!(file.path().as_os_str(), "foo.txt");
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// Set response **Status Code**
    pub fn set_status_code(mut self, status: StatusCode) -> Self {
        self.status_code = status;
        self
    }

    pub(crate) fn etag(&self) -> Option<header::EntityTag> {
        // This etag format is similar to Apache's.
        self.modified.as_ref().map(|mtime| {
            let ino = {
                #[cfg(unix)]
                {
                    self.md.ino()
                }
                #[cfg(not(unix))]
                {
                    0
                }
            };

            let dur = mtime
                .duration_since(UNIX_EPOCH)
                .expect("modification time must be after epoch");
            header::EntityTag::strong(format!(
                "{:x}:{:x}:{:x}:{:x}",
                ino,
                self.md.len(),
                dur.as_secs(),
                dur.subsec_nanos()
            ))
        })
    }

    pub(crate) fn last_modified(&self) -> Option<header::HttpDate> {
        self.modified.map(|mtime| mtime.into())
    }

    pub fn into_response(self, req: &HttpRequest) -> Result<HttpResponse, Error> {
        if self.status_code != StatusCode::OK {
            let mut resp = HttpResponse::build(self.status_code);
            // resp.set(header::ContentType(self.content_type.into()))
            //     .set(header::ContentDisposition(self.content_disposition));
            
            if let Some(current_encoding) = self.encoding {
                resp.encoding(current_encoding);
            }
            let reader = ChunkedReadFile {
                size: self.md.len(),
                offset: 0,
                file: Some(self.file),
                fut: None,
                counter: 0,
            };
            return Ok(resp.streaming(reader));
        }

        let etag = self.etag();
        let last_modified = self.last_modified();

        // check preconditions
        let precondition_failed = if !any_match(etag.as_ref(), req) {
            true
        } else if let (Some(ref m), Some(header::IfUnmodifiedSince(ref since))) =
            (last_modified, req.get_header())
        {
            let t1: SystemTime = m.clone().into();
            let t2: SystemTime = since.clone().into();
            match (t1.duration_since(UNIX_EPOCH), t2.duration_since(UNIX_EPOCH)) {
                (Ok(t1), Ok(t2)) => t1 > t2,
                _ => false,
            }
        } else {
            false
        };

        // check last modified
        let not_modified = if !none_match(etag.as_ref(), req) {
            true
        } else if req.headers().contains_key(&header::IF_NONE_MATCH) {
            false
        } else if let (Some(ref m), Some(header::IfModifiedSince(ref since))) =
            (last_modified, req.get_header())
        {
            let t1: SystemTime = m.clone().into();
            let t2: SystemTime = since.clone().into();
            match (t1.duration_since(UNIX_EPOCH), t2.duration_since(UNIX_EPOCH)) {
                (Ok(t1), Ok(t2)) => t1 <= t2,
                _ => false,
            }
        } else {
            false
        };

        let mut resp = HttpResponse::build(self.status_code);
            // resp.set(header::ContentType(self.content_type.into()))
            //     .set(header::ContentDisposition(self.content_disposition));

        // default compressing
        if let Some(current_encoding) = self.encoding {
            resp.encoding(current_encoding);
        }

        resp.if_some(last_modified, |lm, resp| {
            resp.set(header::LastModified(lm));
        })
        .if_some(etag, |etag, resp| {
            resp.set(header::ETag(etag));
        });

        resp.header(header::ACCEPT_RANGES, "bytes");

        let mut length = self.md.len();
        let mut offset = 0;

        // check for range header
        if let Some(ranges) = req.headers().get(&header::RANGE) {
            if let Ok(rangesheader) = ranges.to_str() {
                if let Ok(rangesvec) = HttpRange::parse(rangesheader, length) {
                    length = rangesvec[0].length;
                    offset = rangesvec[0].start;
                    resp.encoding(ContentEncoding::Identity);
                    resp.header(
                        header::CONTENT_RANGE,
                        format!(
                            "bytes {}-{}/{}",
                            offset,
                            offset + length - 1,
                            self.md.len()
                        ),
                    );
                } else {
                    resp.header(header::CONTENT_RANGE, format!("bytes */{}", length));
                    return Ok(resp.status(StatusCode::RANGE_NOT_SATISFIABLE).finish());
                };
            } else {
                return Ok(resp.status(StatusCode::BAD_REQUEST).finish());
            };
        };

        if precondition_failed {
            return Ok(resp.status(StatusCode::PRECONDITION_FAILED).finish());
        } else if not_modified {
            return Ok(resp.status(StatusCode::NOT_MODIFIED).finish());
        }

        let reader = ChunkedReadFile {
            offset,
            size: length,
            file: Some(self.file),
            fut: None,
            counter: 0,
        };

        if offset != 0 || length != self.md.len() {
            resp.status(StatusCode::PARTIAL_CONTENT);
        }

        Ok(resp.body(SizedStream::new(length, reader)))
    }
}

impl Deref for NamedFile {
    type Target = File;

    fn deref(&self) -> &File {
        &self.file
    }
}

impl DerefMut for NamedFile {
    fn deref_mut(&mut self) -> &mut File {
        &mut self.file
    }
}

/// Returns true if `req` has no `If-Match` header or one which matches `etag`.
fn any_match(etag: Option<&header::EntityTag>, req: &HttpRequest) -> bool {
    match req.get_header::<header::IfMatch>() {
        None | Some(header::IfMatch::Any) => true,
        Some(header::IfMatch::Items(ref items)) => {
            if let Some(some_etag) = etag {
                for item in items {
                    if item.strong_eq(some_etag) {
                        return true;
                    }
                }
            }
            false
        }
    }
}

/// Returns true if `req` doesn't have an `If-None-Match` header matching `req`.
fn none_match(etag: Option<&header::EntityTag>, req: &HttpRequest) -> bool {
    match req.get_header::<header::IfNoneMatch>() {
        Some(header::IfNoneMatch::Any) => false,
        Some(header::IfNoneMatch::Items(ref items)) => {
            if let Some(some_etag) = etag {
                for item in items {
                    if item.weak_eq(some_etag) {
                        return false;
                    }
                }
            }
            true
        }
        None => true,
    }
}

impl Responder for NamedFile {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, req: &HttpRequest) -> Self::Future {
        ready(self.into_response(req))
    }
}
