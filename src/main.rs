use rusqlite::{params, Connection, NO_PARAMS};
use std::time::{Duration, SystemTime};
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
    let rebuild = args.get_bool("rebuild");
    let flac_path = args.get_string("dir");
    let db_path = args.get_string("db");
    let mut v = vec![];

    let mut conn = Connection::open(db_path)?;
    let tx = conn.transaction()?;
    {
        tx.execute(FLACS_SCHEMA, params![])?;
        tx.execute(UPDATES_SCHEMA, params![])?;
        if rebuild {
            tx.execute(TRUNCATE_FLACS_QUERY, params![])?;
            tx.execute(TRUNCATE_UPDATES_QUERY, params![])?;
        }
        let last_update: Option<i64> = tx.query_row(
            "select coalesce(max(timestamp),0) from update_history;",
            NO_PARAMS,
            |row| row.get(0),
        )?;
        let last_update = epoch_to_system_time(last_update.unwrap_or(0) as u64);

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
        let now = system_time_to_epoch(SystemTime::now());
        tx.execute(
            "insert into update_history(timestamp) values (?1)",
            params![now as i64],
        )?;
    }

    tx.commit()?;
    Ok(())
}

fn system_time_to_epoch(st: SystemTime) -> u64 {
    st.duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn epoch_to_system_time(epoch: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH
        .checked_add(Duration::from_secs(epoch))
        .expect("Cannot convert epoch to SystemTime.")
}

fn filter(last_update: SystemTime, entry: &DirEntry) -> bool {
    entry.file_type().is_dir()
        || (entry.file_type().is_file()
            && entry_is_flac(entry)
            && entry_is_modified_after(last_update, entry))
}

fn entry_is_flac(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.to_ascii_uppercase().ends_with("FLAC"))
        .unwrap_or(false)
}
fn entry_is_modified_after(last_update: SystemTime, entry: &DirEntry) -> bool {
    entry
        .metadata()
        .map(|m| {
            m.modified()
                .map(|m| m.duration_since(last_update).is_ok())
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

const HELP_STR: &str = "
Searches for flac files and inserts their metadata to a sqlite database:
Usage: flacdb <dir> <path>
    --rebuild rebuild database from scratch instead of incremental update
    <dir> (string) path to a directory containing flac files
    <db> (string) path to a database file (will be created if it does not exist)";

const FLACS_SCHEMA: &str = "create table if not exists flacs(file_dir, file_path, key, value);";
const TRUNCATE_FLACS_QUERY: &str = "delete from flacs;";
const TRUNCATE_UPDATES_QUERY: &str = "delete from update_history;";
const UPDATES_SCHEMA: &str = "create table if not exists update_history(timestamp);";
