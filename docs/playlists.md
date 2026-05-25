# rtrax Playlists

rtrax uses the standard M3U format — plain text, one file path per line.
Lines starting with `#` are comments or metadata and are skipped on load.

## Loading a Playlist

Pass a `.m3u` file with `--playlist` (`-l`):

```sh
rtrax --playlist my-favourites.m3u
```

Or pass multiple files directly — they become an inline playlist for the
session without touching any file on disk:

```sh
rtrax *.xm
rtrax file1.it file2.s3m file3.xm
```

## The Default Playlist

When no `--playlist` flag is given, `a` appends to the default playlist:

| Platform | Path |
|----------|------|
| Linux    | `~/.local/share/rtrax/playlist.m3u` |
| macOS    | `~/Library/Application Support/rtrax/playlist.m3u` |

The file is created automatically (with an `#EXTM3U` header) on first append
if it doesn't exist yet.

## Adding Songs While Playing

Press `a` to append the currently-playing file to the active playlist.
Each press appends one entry — pressing `a` multiple times is safe.

If a playlist was loaded with `--playlist`, that file is updated.
Otherwise the default playlist is used.

## Navigation

`n` (next) and `p` (previous) follow this priority:

1. The active playlist (loaded via `--playlist` or the inline list from
   multiple `FILES` arguments).
2. The other files in the same folder as the currently-playing file, if no
   playlist is active.

## File Format

A minimal `.m3u` file looks like this:

```
#EXTM3U
/path/to/song.xm
/path/to/another.it
relative/path/works/too.s3m
```

The `#EXTM3U` header is optional for loading but is always written by rtrax.
`#EXTINF` metadata lines are accepted and silently ignored — rtrax reads
titles and durations directly from the module via libopenmpt.

Both absolute and relative paths are supported. Relative paths are resolved
from the directory that contains the `.m3u` file.
