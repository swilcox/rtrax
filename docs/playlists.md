# rtrax Playlists

rtrax uses the standard M3U format — plain text, one file path per line.
Lines starting with `#` are comments or metadata and are skipped on load.

## Two Modes

How you launch rtrax decides what the playlist is *for*:

**Queue mode — play a playlist.** Pass a `.m3u` with `--playlist` (`-l`) and
nothing else:

```sh
rtrax --playlist my-favourites.m3u
```

The left panel becomes the queue: it lists the playlist's tracks, marks the
now-playing one, `n`/`p` and auto-advance walk it, and `/` then `Enter` on a
track jumps straight to it.

**Browse mode — build a playlist.** Pass a file or directory *alongside*
`--playlist`:

```sh
rtrax --playlist favourites.m3u ~/mods
```

Now the left panel is the file browser and `n`/`p` walk the folder; the
playlist is purely the destination for `a`. This is the "audition tracks and
keep the good ones" workflow.

Passing multiple files (no `--playlist`) makes an inline queue for the session
without touching disk; passing a single directory just opens the browser there:

```sh
rtrax *.xm
rtrax file1.it file2.s3m file3.xm
rtrax ~/mods
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

`n` (next) and `p` (previous) follow the active mode:

- **Queue mode:** they walk the playlist (the `--playlist` file, or the inline
  list from multiple `FILES` arguments).
- **Browse mode:** they walk the module files in the browsed folder.

In queue mode, `/` focuses the queue panel and `Enter` jumps straight to the
highlighted track.

## Shuffle

Press `z` (or launch with `--shuffle` / `-z`) to randomize play order. It
applies to the active collection — the playlist in queue mode, the folder's
modules in browse mode. Toggling shuffle on keeps the current track playing and
shuffles the rest; toggling off restores the natural order. A `⤮ shuffle` marker
appears on the status line and in the panel title while it's active.

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
