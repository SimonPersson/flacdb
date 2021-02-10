use rusqlite::{params, Connection};
use walkdir::{DirEntry, WalkDir};

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
        tx.execute(FLACS_SCHEMA, params![])?;
        tx.execute(TRUNCATE_FLACS_QUERY, params![])?;

        let mut stmt = tx.prepare(
            "insert into flacs(file_dir, file_path, key, value) values (?1, ?2, ?3, ?4)",
        )?;

        for file in WalkDir::new(flac_path)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| filter(last_update, e))
            .filter(|re| {
                re.as_ref()
                    .map(|e| e.file_type().is_file())
                    .unwrap_or(false)
            })
        {
            if let Ok(file) = file {
                let mut vorbis_comments = metaflac::read_from(file.path().into(), &mut v)?;
                while let Ok(Some((key, val))) = vorbis_comments.next(&v) {
                    stmt.execute(params![
                        file.path()
                            .parent()
                            .map(|p| p.to_string_lossy())
                            .unwrap_or_else(|| "".into()),
                        file.path().to_string_lossy(),
                        key,
                        val
                    ])?;
                }
            }
        }
    }

    tx.commit()?;
    Ok(())
}

fn filter(entry: &DirEntry) -> bool {
    entry.file_type().is_dir() || (entry.file_type().is_file() && entry_is_flac(entry))
}

fn entry_is_flac(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.to_ascii_uppercase().ends_with("FLAC"))
        .unwrap_or(false)
}

const HELP_STR: &str = "
Searches directory for flac files and inserts their metadata to a sqlite database:
Usage: flacdb --db flacdb.sqlite <dir>
    <dir> (string) directories containing flac files
    <db> (string) path to a database file (will be created if it does not exist)";

const FLACS_SCHEMA: &str = "create table if not exists flacs(file_dir, file_path, key, value);";
const TRUNCATE_FLACS_QUERY: &str = "delete from flacs;";
