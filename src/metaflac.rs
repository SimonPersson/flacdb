use crate::error::Error;
use std::convert::TryInto;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

pub fn read_from(path: PathBuf, data: &mut Vec<u8>) -> Result<VorbisComment, Error> {
    let mut reader = File::open(&*path)?;
    let mut ident = [0; 4];
    reader.read_exact(&mut ident)?;

    if &ident != b"fLaC" {
        return Err(Error::InvalidFlacHeader(path));
    }

    read_tags(&mut reader, path, data)
}

// See documentation: https://xiph.org/flac/format.html
fn read_tags(reader: &mut File, path: PathBuf, data: &mut Vec<u8>) -> Result<VorbisComment, Error> {
    loop {
        let mut buf = [0; 4];

        reader.read_exact(&mut buf)?;
        let is_last = (buf[0] & 0b1000_0000) != 0;
        let blocktype_byte = buf[0] & 0b0111_1111;
        let length = u32::from_be_bytes(buf) & 0x00FF_FFFF;

        if blocktype_byte == 4 {
            data.clear();
            reader.take(u64::from(length)).read_to_end(data)?;
            return Ok(VorbisComment::from_bytes(path, data));
        } else if is_last {
            return Ok(VorbisComment::empty(path));
        } else {
            reader.seek(SeekFrom::Current(i64::from(length)))?;
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct VorbisComment {
    pub path: PathBuf,
    num_comments: u32,
    i: usize,
    curr: u32,
}

impl VorbisComment {
    pub fn empty(path: PathBuf) -> VorbisComment {
        VorbisComment {
            path,
            num_comments: 0,
            i: 0,
            curr: 1,
        }
    }

    pub fn from_bytes(path: PathBuf, bytes: &[u8]) -> VorbisComment {
        let vendor_length = u32::from_le_bytes((&bytes[0..4]).try_into().unwrap()) as usize;
        let num_comments = u32::from_le_bytes(
            (&bytes[4 + vendor_length..8 + vendor_length])
                .try_into()
                .unwrap(),
        );
        VorbisComment {
            path,
            num_comments,
            i: 8 + vendor_length,
            curr: 0,
        }
    }

    pub fn next<'a, 'b>(
        &'a mut self,
        bytes: &'b [u8],
    ) -> Result<Option<(&'b str, &'b str)>, Error> {
        if self.curr < self.num_comments {
            let comment_length =
                u32::from_le_bytes((bytes[self.i..self.i + 4]).try_into().unwrap()) as usize;

            let (key, value) =
                read_vorbis_comment(&bytes[self.i + 4..self.i + 4 + comment_length])?;

            self.curr += 1;
            self.i += comment_length + 4;
            Ok(Some((key, value)))
        } else {
            Ok(None)
        }
    }
}

fn read_vorbis_comment(bytes: &[u8]) -> Result<(&str, &str), Error> {
    let comments = std::str::from_utf8(bytes)?;

    let mut comments_split = comments.split('=');
    let key = comments_split
        .next()
        .ok_or_else(|| Error::MalformedVorbisComment(comments.into()))?;
    let value = comments_split
        .next()
        .ok_or_else(|| Error::MalformedVorbisComment(comments.into()))?;
    Ok((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_all_tags() {
        let path = PathBuf::from("test-data/test-tag.flac");
        let mut buf = Vec::new();
        let mut vorbis_comments = read_from(path.clone(), &mut buf).unwrap();

        assert_eq!(Some(("TEST", "1")), vorbis_comments.next(&*buf).unwrap());
        assert_eq!(Some(("TEST", "2")), vorbis_comments.next(&*buf).unwrap());
        assert_eq!(None, vorbis_comments.next(&*buf).unwrap());
    }
}
