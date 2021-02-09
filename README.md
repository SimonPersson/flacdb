# flacdb
Searches directory for flac files and inserts their metadata to a sqlite database:

Usage:
```
flacdb <dir> <path>
    --rebuild rebuild database from scratch instead of incremental update
    <dir> (string) path to a directory containing flac files
    <db> (string) path to a database file (will be created if it does not exist)
```

The music collection can now be queried in all sorts of ways. Say you want to find a rare jazz record with John Coltrane on sax and "Shadow" Wilson on drums. Easy:
```
sqlite> select file_dir from flacs where lower(value) like '%shadow%drums%' and key = 'PERFORMER'
intersect
select file_dir from flacs where lower(value) like '%coltrane%sax%' and key = 'PERFORMER';
/mnt/storage/music/Monk, Thelonious & Coltrane, John/Thelonious Monk with John Coltrane
sqlite>
```
