use lapp;
use rusqlite::{params, Connection};
use walkdir;

mod error;
mod metaflac;

fn main() {
    match run() {
        Ok(_) => (),
        Err(e) => println!("{}", e),
    }
}

fn run() -> Result<(), error::Error> {
    let args = lapp::parse_args(HELP_STR);
    let flac_path = args.get_string("dir");
    let db_path = args.get_string("db");
    let mut v = vec![];

    let mut conn = Connection::open(db_path)?;
    let tx = conn.transaction()?;
    {
        tx.execute(SCHEMA, params![])?;
        let mut stmt = tx.prepare("insert into flacs(path, key, value) values (?1, ?2, ?3)")?;

        for file in walkdir::WalkDir::new(flac_path)
            .follow_links(true)
            .into_iter()
            .filter(is_flac_file)
        {
            if let Ok(file) = file {
                let mut vorbis_comments = metaflac::read_from(file.path().into(), &mut v)?;
                while let Ok(Some((key, val))) = vorbis_comments.next(&v) {
                    stmt.execute(params![file.path().to_string_lossy(), key, val])?;
                }
            }
        }
    }

    tx.commit()?;
    Ok(())
}

fn is_flac_file(entry: &Result<walkdir::DirEntry, walkdir::Error>) -> bool {
    if let Ok(entry) = entry {
        entry
            .file_name()
            .to_str()
            .map(|s| s.ends_with("flac"))
            .unwrap_or(false)
    } else {
        false
    }
}

const HELP_STR: &'static str = "
Searches for flac files and inserts their metadata to a sqlite database:
Usage: flacdb <dir> <path>
    <dir> (string) path to a directory containing flac files
    <db> (string) path to a database file (will be created if it does not exist)";

const SCHEMA: &'static str = "create table if not exists flacs(path, key, value); truncate flacs;";
