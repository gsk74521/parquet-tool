# parquet-tool

[中文版](README.md) | **English**

A Parquet file inspection tool written in Rust, compiled into a single self-contained executable with **no runtime dependencies**. Available for Linux (statically linked musl) and Windows.

## Features

- **schema** — print the schema of a parquet file (field names, physical/logical types, nullability, nested structure)
- **rowcount** — print the number of rows in a parquet file
- **show** — print the contents with table / CSV / JSON output, row limits, offset, and column filtering
- **cat** — print the entire contents (all rows)
- **head** — print the first N rows (like the Linux `head` command)
- **compression** — print per-column and per-file compression statistics (before/after sizes, codec, ratio)

## Usage

```
parquet-tool <COMMAND> [OPTIONS]

Commands:
  schema       Print the file schema (includes row count, row group count, creator)
  rowcount     Print the total row count
  show         Print the file contents (limited rows)
  cat          Print the entire contents (all rows)
  head         Print the first N rows
  compression  Print compression statistics (per-column ratio)
```

### Show schema

```
parquet-tool schema <file.parquet>
```

Example output:

```
File: sample.parquet
Row count: 100
Row groups: 1
Created by: parquet-rs version 60.0.0

arrow_schema: message
  id: INT64 [REQUIRED]
  name: BYTE_ARRAY (String) [OPTIONAL]
  score: DOUBLE [OPTIONAL]
  active: BOOLEAN [OPTIONAL]
  ts: INT64 (Timestamp(TimestampType { is_adjusted_to_u_t_c: true, unit: MICROS })) [OPTIONAL]
  address: group [OPTIONAL]
    city: BYTE_ARRAY (String) [OPTIONAL]
    zip: INT32 [REQUIRED]
  tags: group [OPTIONAL]
    list: group [REPEATED]
      item: BYTE_ARRAY (String) [OPTIONAL]
```

### Show row count

```
parquet-tool rowcount <file.parquet>
# Output: 100
```

### Show contents

```
# Table output (default), first 20 rows
parquet-tool show <file.parquet>

# Only the first 5 rows
parquet-tool show <file.parquet> --limit 5

# Skip the first 98 rows, print from row 99
parquet-tool show <file.parquet> --offset 98 --limit 5

# Only selected columns
parquet-tool show <file.parquet> --columns id,name,score --limit 5

# CSV output
parquet-tool show <file.parquet> --format csv --limit 5

# JSON output
parquet-tool show <file.parquet> --format json --limit 5

# Show all rows (--limit 0 means unlimited)
parquet-tool show <file.parquet> --limit 0
```

### Options (show)

| Option | Description | Default |
|--------|-------------|---------|
| `-l, --limit <N>` | Maximum number of rows to print, `0` = all | `20` |
| `-o, --offset <N>` | Number of rows to skip before printing | `0` |
| `-f, --format <fmt>` | Output format: `table` / `csv` / `json` | `table` |
| `-c, --columns <cols>` | Only these columns (comma separated), e.g. `id,name` | all |

### cat — print the entire contents (all rows)

```
# All rows as a table
parquet-tool cat <file.parquet>

# All rows as CSV / JSON
parquet-tool cat <file.parquet> --format csv
parquet-tool cat <file.parquet> --format json

# Only selected columns
parquet-tool cat <file.parquet> --columns id,name
```

`cat` prints all rows and is equivalent to `show --limit 0`. CSV / JSON output is streamed, so even very large files are not fully loaded into memory.

### head — print the first N rows

```
# First 10 rows (default)
parquet-tool head <file.parquet>

# First 3 rows
parquet-tool head <file.parquet> -n 3
parquet-tool head <file.parquet> --lines 3

# CSV / JSON / selected columns
parquet-tool head <file.parquet> -n 5 --format csv
parquet-tool head <file.parquet> -n 5 --format json --columns id,name
```

### Options (cat / head)

| Command | Option | Description | Default |
|---------|--------|-------------|---------|
| `cat` | `-f, --format <fmt>` | Output format: `table` / `csv` / `json` | `table` |
| `cat` | `-c, --columns <cols>` | Only these columns | all |
| `head` | `-n, --lines <N>` | Number of rows to print | `10` |
| `head` | `-f, --format <fmt>` | Output format: `table` / `csv` / `json` | `table` |
| `head` | `-c, --columns <cols>` | Only these columns | all |

### compression — compression statistics

```
parquet-tool compression <file.parquet>
```

Example output (one row per column + two summary rows):

```
File: sample_zstd.parquet
Row count: 100000
Row groups: 1

column                         uncompressed     compressed          codec      ratio
------------------------------------------------------------------------------------
rg0."id"                            1002600         296651 ZSTD(ZstdLevel(1))      3.38x
rg0."name"                          1549242         173844 ZSTD(ZstdLevel(1))      8.91x
rg0."score"                           88632           2225 ZSTD(ZstdLevel(1))     39.83x
rg0."active"                          12600            200 ZSTD(ZstdLevel(1))     63.00x
rg0."city"                            75871           1645 ZSTD(ZstdLevel(1))     46.12x
rg0."zip"                             88232           2235 ZSTD(ZstdLevel(1))     39.48x
------------------------------------------------------------------------------------
TOTAL (data only)                   2817177         476800                     5.91x
TOTAL (incl. footer)                2817177         479116                     5.88x
```

Notes:
- `uncompressed` / `compressed` are the bytes of all data pages in a column before / after compression (read from the file footer metadata; **no data is decompressed**, so it is fast)
- `ratio` = `uncompressed / compressed`; values > 1 mean compression is effective. Columns with very little data or low regularity may show a value slightly below 1 (codec overhead outweighs the gain — normal)
- The two summary rows: `TOTAL (data only)` sums column data only; `TOTAL (incl. footer)` is computed against the full file size and reflects real disk usage (including footer, page headers, etc.)

**Important: `uncompressed` is the size AFTER dictionary/RLE encoding and BEFORE the compression codec runs — NOT the raw logical data size.**

Parquet performs **two stages of compression**: first dictionary encoding (stage 1, usually the dominant one), then the compression codec (stage 2). For highly repeated data, dictionary encoding already collapses the column to a tiny size, so the codec (stage 2) has almost nothing to compress and only adds its per-page overhead — this is why `ratio` may be < 1. It does **NOT** mean the file is stored inefficiently. To judge real storage efficiency, look at the overall file size (compared with the raw logical data size).

## Supported data types

Supports all Parquet physical and logical types, including:
- Primitive: INT32 / INT64 / INT96, FLOAT / DOUBLE, BOOLEAN, BYTE_ARRAY, FIXED_LEN_BYTE_ARRAY
- Logical: String, Decimal, Date, Time, Timestamp (including IANA timezones such as UTC), JSON, UUID, etc.
- Nested: Struct (group), List (list/repeated), etc.
- Compression codecs: Snappy, Gzip, Brotli, LZ4, Zstd (all supported for reading)

NULL values are shown as `NULL` in table/CSV output and as `null` in JSON.

## Building

### Windows

Requires the Rust toolchain (rustc/cargo 1.75+).

```
cargo build --release
```

Artifact: `target/release/parquet-tool.exe`

Note: on Windows, if gcc fails to compile zstd C sources due to a temp-directory permission error, point the `TMP`/`TEMP` environment variables to a writable directory first.

### Linux (static musl, cross-compiled)

Requires rustup + [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) (`pip install cargo-zigbuild`, bundles zig).

```
rustup target add x86_64-unknown-linux-musl
cargo zigbuild --release --target x86_64-unknown-linux-musl
```

Artifact: `target/x86_64-unknown-linux-musl/release/parquet-tool`

The artifact is a fully statically linked ELF executable that runs on any x86_64 Linux:

```bash
chmod +x parquet-tool
./parquet-tool head file.parquet
```

## Test data

`examples/gen_sample.rs` generates a sample file with a variety of types, nested structures, and NULLs:

```
cargo run --release --example gen_sample -- sample.parquet 100
```

## Project structure

- `src/main.rs` — the main program
- `examples/gen_sample.rs` — test-data generator
- `Cargo.toml` — dependencies (parquet 60 / arrow 60 / clap / comfy-table)

## License

[MIT License](LICENSE) — see the LICENSE file in the repository root.

## Publishing to GitHub Releases

The repository includes an automated release workflow [.github/workflows/release.yml](.github/workflows/release.yml). Pushing a `v*` tag automatically cross-compiles the Linux (static musl) and Windows binaries, packages them (tar.gz / zip), generates SHA256 checksums, and creates a GitHub Release.

**First release (one-time):**
```bash
# 1. Create an empty repository on GitHub (do NOT initialize with README/.gitignore)
# 2. Add the remote and push
git remote add origin https://github.com/<your-username>/parquet-tool.git
git branch -M main
git add -A && git commit -m "Initial release"
git push -u origin main
```

**Release a new version (every subsequent time):**
```bash
# 1. Bump the version in Cargo.toml (e.g. 1.1.0)
# 2. Commit and tag
git add -A && git commit -m "Release 1.1.0"
git tag v1.1.0
git push origin main --tags
# GitHub Actions builds automatically and creates a Release with tar.gz/zip/SHA256SUMS
```

**Release management tips:**
- **Versioning**: use `v<version>` tags (e.g. `v1.0.0`), kept consistent with the `version` in Cargo.toml
- **Semantic versioning**: `MAJOR.MINOR.PATCH` — bump MAJOR for breaking changes, MINOR for new features, PATCH for bug fixes
- **Release notes**: the workflow uses `generate_release_notes: true`; you can also add notes manually on the GitHub page after publishing
- **Rollback**: keep historical tags/releases so older versions can be downloaded; use `git revert` for code rollback
- **Check status**: watch the Actions tab on the repository page; click into the logs on failure to debug

## Publishing to PyPI (`pip install parquet-tool`)

The repository includes a PyPI publishing workflow [.github/workflows/publish-pypi.yml](.github/workflows/publish-pypi.yml). It builds Linux (statically linked musllinux) and Windows wheels so users can install with `pip install parquet-tool` and get the `parquet-tool` command.

**One-time setup:**
1. Create an account at [pypi.org](https://pypi.org)
2. Configure either Trusted Publishing (recommended) or an API token:
   - **Trusted Publishing** (recommended): PyPI → Account settings → Publishing → add the `gsk74521/parquet-tool` repository with the `pypi` environment
   - **API token**: PyPI → Account settings → API tokens → create a token, then add it as `PYPI_API_TOKEN` in the repository Settings → Secrets and variables → Actions

**Release a new version:**
```bash
# Manually trigger: GitHub Actions page → publish-pypi → Run workflow
# Or push a v* tag (shared with GitHub Releases)
git push origin main --tags
```

After publishing, users can install with: `pip install parquet-tool`
