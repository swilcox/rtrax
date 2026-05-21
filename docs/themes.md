# rtrax Themes

rtrax themes are TOML files that define the colors used by the terminal UI.
Themes can be selected at startup through the main config file, and custom
themes are included when cycling themes with `t`.

## Config Location

rtrax looks for config in this order:

1. `$XDG_CONFIG_HOME/rtrax`
2. `~/.config/rtrax`
3. The platform config directory

The main config file is:

```text
~/.config/rtrax/config.toml
```

Custom themes live in:

```text
~/.config/rtrax/themes/
```

## Selecting a Theme

Set `theme` in `config.toml`:

```toml
theme = "neon-blue"
```

Built-in themes are:

- `default`
- `high-contrast`
- `sixteen`

Custom themes are selected by file stem. For example, this file:

```text
~/.config/rtrax/themes/neon-blue.toml
```

is selected with:

```toml
theme = "neon-blue"
```

## Theme Files

A theme file can override any subset of color keys. Missing keys inherit from
the base theme.

```toml
# ~/.config/rtrax/themes/neon-blue.toml
extends = "default"

bg = "reset"
fg = "#d8f7ff"
fg_dim = "#5a8faa"
border = "#164866"
border_focus = "#00ccff"
accent = "#33f6ff"
note = "#8fefff"
instrument = "#4cb8ff"
volume = "#6ce7ff"
effect = "#b6f4ff"
meter_low = "#168dff"
meter_mid = "#22d8ff"
meter_high = "#e2fbff"
current_row_bg = "#06283b"
```

`extends` is optional. If omitted, the theme starts from `default`.

`extends` can reference a built-in theme:

```toml
extends = "sixteen"
```

or another custom theme:

```toml
extends = "neon-blue"
```

## Color Keys

| Key              | Used for                                  |
|------------------|--------------------------------------------|
| `bg`             | General background                         |
| `fg`             | Primary foreground text                    |
| `fg_dim`         | Dim text, secondary labels, inactive hints |
| `border`         | Normal panel borders                       |
| `border_focus`   | Focused panel borders                      |
| `accent`         | Active state, titles, highlights           |
| `note`           | Pattern note values                        |
| `instrument`     | Pattern instrument values                  |
| `volume`         | Pattern volume column values               |
| `effect`         | Pattern effect column values               |
| `meter_low`      | Low meter range                            |
| `meter_mid`      | Mid meter range                            |
| `meter_high`     | Hot meter range                            |
| `current_row_bg` | Current pattern row background             |

## Color Values

Colors can be written as truecolor hex:

```toml
accent = "#33f6ff"
```

or as terminal color names:

```toml
accent = "light-cyan"
fg_dim = "dark-gray"
bg = "reset"
```

Supported names are:

- `reset`
- `black`
- `red`
- `green`
- `yellow`
- `blue`
- `magenta`
- `cyan`
- `gray`
- `dark-gray`
- `light-red`
- `light-green`
- `light-yellow`
- `light-blue`
- `light-magenta`
- `light-cyan`
- `white`

Underscores and spaces are accepted in color names, so `dark_gray`,
`dark gray`, and `dark-gray` are equivalent.

## Cycling Themes

Press `t` in rtrax to cycle through:

1. `default`
2. `high-contrast`
3. `sixteen`
4. Custom `.toml` files found in the themes directory

Custom themes are discovered at startup. Restart rtrax after adding or renaming
a theme file.

## Minimal Theme

This is enough to create a custom accent variant:

```toml
# ~/.config/rtrax/themes/ice.toml
extends = "default"

accent = "#7df9ff"
border_focus = "#4fdfff"
current_row_bg = "#092635"
```

Then select it with:

```toml
theme = "ice"
```
