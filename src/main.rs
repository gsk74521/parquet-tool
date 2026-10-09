use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};

use arrow::array::Array;
use arrow::record_batch::RecordBatch;
use arrow::util::display::array_value_to_string;
use clap::{Parser, Subcommand};
use comfy_table::{presets::UTF8_FULL, Cell, Table};
use parquet::arrow::arrow_reader::{ParquetRecordBatchReader, ParquetRecordBatchReaderBuilder};
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::schema::types::Type;

#[derive(Parser)]
#[command(
    name = "parquet-tool",
    version,
    about = "Inspect Apache Parquet files: schema, row count, and contents"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the schema of the parquet file
    Schema {
        /// Path to the parquet file
        file: PathBuf,
    },
    /// Print the number of rows in the file
    Rowcount {
        /// Path to the parquet file
        file: PathBuf,
    },
    /// Print the contents of the file (limited rows)
    Show {
        /// Path to the parquet file
        file: PathBuf,
        /// Maximum number of rows to print (0 = all rows)
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
        /// Number of rows to skip before printing
        #[arg(short, long, default_value_t = 0)]
        offset: usize,
        /// Output format: table, csv, json
        #[arg(short, long, default_value = "table")]
        format: String,
        /// Only show these columns (comma separated)
        #[arg(short, long)]
        columns: Option<String>,
    },
    /// Print the entire contents of the file (all rows)
    Cat {
        /// Path to the parquet file
        file: PathBuf,
        /// Output format: table, csv, json
        #[arg(short, long, default_value = "table")]
        format: String,
        /// Only show these columns (comma separated)
        #[arg(short, long)]
        columns: Option<String>,
    },
    /// Print the first N rows of the file
    Head {
        /// Path to the parquet file
        file: PathBuf,
        /// Number of rows to print
        #[arg(short = 'n', long = "lines", default_value_t = 10)]
        lines: usize,
        /// Output format: table, csv, json
        #[arg(short, long, default_value = "table")]
        format: String,
        /// Only show these columns (comma separated)
        #[arg(short, long)]
        columns: Option<String>,
    },
    /// Count distinct (unique) values per column
    Distinct {
        /// Path to the parquet file
        file: PathBuf,
        /// Only analyze these columns (comma separated)
        #[arg(short, long)]
        columns: Option<String>,
    },
    /// Print per-column and per-file compression statistics
    #[command(
        about = "Print per-column and per-file compression statistics",
        long_about = concat!(
            "Print per-column and per-file compression statistics (read from footer metadata,\n",
            "does NOT decompress any data).\n",
            "\n",
            "Columns shown per column:\n",
            "  uncompressed  bytes after Parquet encoding (dictionary/RLE/plain) and BEFORE the\n",
            "                compression codec runs\n",
            "  compressed    bytes AFTER the compression codec runs\n",
            "  codec         compression algorithm used (SNAPPY/GZIP/BROTLI/LZ4/ZSTD/UNCOMPRESSED)\n",
            "  ratio         uncompressed / compressed\n",
            "\n",
            "IMPORTANT: 'uncompressed' is the size after dictionary/RLE encoding, NOT the raw\n",
            "logical data size. Dictionary encoding is the first, usually dominant, compression\n",
            "stage in Parquet. For highly repeated data it already collapses the column to a tiny\n",
            "size, so 'ratio' may be < 1 (the codec then only adds its per-page overhead). That is\n",
            "normal and does NOT mean the file is stored inefficiently -- check the overall file\n",
            "size to judge real storage efficiency.\n",
            "\n",
            "Summary rows:\n",
            "  TOTAL (data only)       sum over column data only\n",
            "  TOTAL (incl. footer)    vs the full file size (includes footer/metadata overhead)"
        )
    )]
    Compression {
        /// Path to the parquet file
        file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match &cli.command {
        Command::Schema { file } => run_schema(file),
        Command::Rowcount { file } => run_rowcount(file),
        Command::Show {
            file,
            limit,
            offset,
            format,
            columns,
        } => {
            let max_rows = if *limit == 0 { None } else { Some(*limit) };
            run_output(file, columns.as_deref(), *offset, max_rows, format)
        }
        Command::Cat {
            file,
            format,
            columns,
        } => run_output(file, columns.as_deref(), 0, None, format),
        Command::Head {
            file,
            lines,
            format,
            columns,
        } => run_output(file, columns.as_deref(), 0, Some(*lines), format),
        Command::Compression { file } => run_compression(file),
        Command::Distinct { file, columns } => run_distinct(file, columns.as_deref()),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn open_reader(path: &Path) -> Result<ParquetRecordBatchReaderBuilder<File>, Box<dyn Error>> {
    let file = File::open(path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
    Ok(builder)
}

fn run_schema(path: &Path) -> Result<(), Box<dyn Error>> {
    let builder = open_reader(path)?;
    let metadata = builder.metadata();
    let file_meta = metadata.file_metadata();
    let num_rows = file_meta.num_rows();
    let num_row_groups = metadata.num_row_groups();
    let created_by = file_meta.created_by().unwrap_or("unknown");

    println!("File: {}", path.display());
    println!("Row count: {num_rows}");
    println!("Row groups: {num_row_groups}");
    println!("Created by: {created_by}");
    println!();

    let schema = file_meta.schema_descr();
    print_type(schema.root_schema(), 0);
    Ok(())
}

fn print_type(t: &Type, depth: usize) {
    let indent = "  ".repeat(depth);
    let basic_info = t.get_basic_info();
    let name = basic_info.name();
    let repetition = if basic_info.has_repetition() {
        format!("[{}]", basic_info.repetition())
    } else {
        String::new()
    };
    if t.is_primitive() {
        let physical = t.get_physical_type();
        let logical = basic_info
            .logical_type_ref()
            .map(|l| format!("{:?}", l))
            .unwrap_or_default();
        let extra = if logical.is_empty() {
            format!("{physical}")
        } else {
            format!("{physical} ({logical})")
        };
        println!("{indent}{name}: {extra} {repetition}");
    } else {
        if t.is_schema() {
            println!("{indent}{name}: message");
        } else {
            println!("{indent}{name}: group {repetition}");
        }
        for child in t.get_fields() {
            print_type(child, depth + 1);
        }
    }
}

fn run_rowcount(path: &Path) -> Result<(), Box<dyn Error>> {
    let builder = open_reader(path)?;
    let num_rows = builder.metadata().file_metadata().num_rows();
    println!("{num_rows}");
    Ok(())
}

/// Count distinct (non-null unique) values per column, like SQL COUNT(DISTINCT col).
/// Streams row groups batch by batch, keeping only a HashSet<OwnedRow> per column,
/// so memory use is proportional to the number of distinct values, not total rows.
fn run_distinct(path: &Path, columns: Option<&str>) -> Result<(), Box<dyn Error>> {
    let builder = open_reader(path)?;
    let schema = builder.schema();

    let selected: Vec<usize> = match columns {
        Some(spec) => spec
            .split(',')
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .map(|c| {
                schema
                    .index_of(c)
                    .map_err(|_| format!("unknown column: {c}").into())
            })
            .collect::<Result<Vec<_>, Box<dyn Error>>>()?,
        None => (0..schema.fields().len()).collect(),
    };

    let names: Vec<String> = selected
        .iter()
        .map(|&i| schema.field(i).name().clone())
        .collect();

    // One RowConverter per column: encodes a single column's values into a
    // comparable/hashable row format, supporting all Arrow data types.
    let converters: Vec<arrow::row::RowConverter> = selected
        .iter()
        .map(|&i| {
            arrow::row::RowConverter::new(vec![arrow::row::SortField::new(
                schema.field(i).data_type().clone(),
            )])
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut distinct_sets: Vec<std::collections::HashSet<arrow::row::OwnedRow>> =
        vec![std::collections::HashSet::new(); selected.len()];
    let mut null_counts: Vec<u64> = vec![0; selected.len()];
    let mut total_rows: u64 = 0;

    let reader = builder.build()?;
    for batch in reader {
        let batch: RecordBatch = batch?;
        let num_rows = batch.num_rows();
        for (k, &col_idx) in selected.iter().enumerate() {
            let arr = batch.column(col_idx);
            let rows = converters[k].convert_columns(std::slice::from_ref(arr))?;
            for (row_idx, row) in rows.iter().enumerate() {
                if arr.is_null(row_idx) {
                    null_counts[k] += 1;
                } else {
                    distinct_sets[k].insert(row.owned());
                }
            }
        }
        total_rows += num_rows as u64;
    }

    println!("File: {}", path.display());
    println!("Row count: {total_rows}");
    println!();
    println!("{:<28} {:>10} {:>10} {:>12}", "column", "distinct", "nulls", "distinct%");
    println!("{}", "-".repeat(66));
    for (k, name) in names.iter().enumerate() {
        let distinct = distinct_sets[k].len();
        let nulls = null_counts[k];
        let pct = if total_rows > 0 {
            distinct as f64 * 100.0 / total_rows as f64
        } else {
            0.0
        };
        println!(
            "{:<28} {:>10} {:>10} {:>11.2}%",
            name, distinct, nulls, pct
        );
    }
    println!("{}", "-".repeat(66));
    println!("distinct  = number of unique non-null values");
    println!("nulls     = number of NULL values");
    println!("distinct% = distinct / total rows");
    Ok(())
}

/// Print per-column and per-file compression statistics.
/// Reads the footer metadata (uncompressed/compressed sizes per column chunk),
/// so it does not decompress any data.
fn run_compression(path: &Path) -> Result<(), Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = SerializedFileReader::new(file)?;
    let meta = reader.metadata();
    let file_meta = meta.file_metadata();

    let mut file_unc: u64 = 0;
    let mut file_cmp: u64 = 0;

    println!("File: {}", path.display());
    println!("Row count: {}", file_meta.num_rows());
    println!("Row groups: {}", meta.num_row_groups());
    println!();
    println!(
        "{:<28} {:>14} {:>14} {:>14} {:>10}",
        "column", "uncompressed", "compressed", "codec", "ratio"
    );
    println!("{}", "-".repeat(84));

    for (rg_idx, rg) in meta.row_groups().iter().enumerate() {
        for col in rg.columns() {
            let unc = col.uncompressed_size() as u64;
            let cmp = col.compressed_size() as u64;
            file_unc += unc;
            file_cmp += cmp;
            let ratio = if cmp > 0 { unc as f64 / cmp as f64 } else { 0.0 };
            let name = format!("rg{rg_idx}.{}", col.column_path());
            println!(
                "{:<28} {:>14} {:>14} {:>14} {:>9.2}x",
                name, unc, cmp, col.compression(), ratio
            );
        }
    }

    println!("{}", "-".repeat(84));
    let data_ratio = if file_cmp > 0 {
        file_unc as f64 / file_cmp as f64
    } else {
        0.0
    };
    println!(
        "{:<28} {:>14} {:>14} {:>14} {:>9.2}x",
        "TOTAL (data only)", file_unc, file_cmp, "", data_ratio
    );

    let raw_file_size = std::fs::metadata(path)?.len();
    let file_ratio = if raw_file_size > 0 {
        file_unc as f64 / raw_file_size as f64
    } else {
        0.0
    };
    println!(
        "{:<28} {:>14} {:>14} {:>14} {:>9.2}x",
        "TOTAL (incl. footer)",
        file_unc,
        raw_file_size,
        "",
        file_ratio
    );
    Ok(())
}

/// Run a query that prints selected columns/rows.
/// `max_rows`: `None` = all rows, `Some(n)` = at most n rows.
fn run_output(
    path: &Path,
    columns: Option<&str>,
    offset: usize,
    max_rows: Option<usize>,
    format: &str,
) -> Result<(), Box<dyn Error>> {
    let builder = open_reader(path)?;
    let schema = builder.schema();

    let selected: Vec<usize> = match columns {
        Some(spec) => spec
            .split(',')
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .map(|c| {
                schema
                    .index_of(c)
                    .map_err(|_| format!("unknown column: {c}").into())
            })
            .collect::<Result<Vec<_>, Box<dyn Error>>>()?,
        None => (0..schema.fields().len()).collect(),
    };

    let header: Vec<String> = selected
        .iter()
        .map(|&i| schema.field(i).name().clone())
        .collect();

    let reader = builder.build()?;

    match format {
        "table" => {
            let rows = collect_rows(reader, &selected, offset, max_rows)?;
            print_table(&header, &rows);
        }
        "csv" => {
            print_csv_stream(reader, &header, &selected, offset, max_rows)?;
        }
        "json" => {
            print_json_stream(reader, &header, &selected, offset, max_rows)?;
        }
        other => {
            return Err(format!(
                "unsupported format: {other} (expected table, csv, json)"
            )
            .into())
        }
    }
    Ok(())
}

/// A row is the string representation of the selected columns.
type Row = Vec<String>;

/// Materialize up to `max_rows` rows (or all rows when `None`),
/// skipping the first `offset` rows. Only used by the `table` format.
fn collect_rows(
    reader: ParquetRecordBatchReader,
    selected: &[usize],
    offset: usize,
    max_rows: Option<usize>,
) -> Result<Vec<Row>, Box<dyn Error>> {
    let mut rows: Vec<Row> = Vec::new();
    let mut skipped: usize = 0;
    let mut emitted: usize = 0;

    for batch in reader {
        let batch: RecordBatch = batch?;
        let num_rows = batch.num_rows();
        for row_idx in 0..num_rows {
            if skipped < offset {
                skipped += 1;
                continue;
            }
            if let Some(m) = max_rows {
                if emitted >= m {
                    return Ok(rows);
                }
            }
            rows.push(extract_row(&batch, selected, row_idx)?);
            emitted += 1;
        }
        if let Some(m) = max_rows {
            if emitted >= m {
                break;
            }
        }
    }
    Ok(rows)
}

fn extract_row(batch: &RecordBatch, selected: &[usize], row_idx: usize) -> Result<Row, Box<dyn Error>> {
    let mut vals = Vec::with_capacity(selected.len());
    for &i in selected {
        let arr = batch.column(i);
        if arr.is_null(row_idx) {
            vals.push("NULL".to_string());
        } else {
            vals.push(array_value_to_string(arr.as_ref(), row_idx)?);
        }
    }
    Ok(vals)
}

/// Iterate rows and call `f` for each, honoring offset/max_rows.
fn for_each_row<F>(
    reader: ParquetRecordBatchReader,
    selected: &[usize],
    offset: usize,
    max_rows: Option<usize>,
    mut f: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(&Row) -> Result<(), Box<dyn Error>>,
{
    let mut skipped: usize = 0;
    let mut emitted: usize = 0;

    for batch in reader {
        let batch: RecordBatch = batch?;
        let num_rows = batch.num_rows();
        for row_idx in 0..num_rows {
            if skipped < offset {
                skipped += 1;
                continue;
            }
            if let Some(m) = max_rows {
                if emitted >= m {
                    return Ok(());
                }
            }
            let row = extract_row(&batch, selected, row_idx)?;
            f(&row)?;
            emitted += 1;
        }
        if let Some(m) = max_rows {
            if emitted >= m {
                break;
            }
        }
    }
    Ok(())
}

fn print_csv_stream(
    reader: ParquetRecordBatchReader,
    header: &[String],
    selected: &[usize],
    offset: usize,
    max_rows: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        header
            .iter()
            .map(|h| csv_escape(h))
            .collect::<Vec<_>>()
            .join(",")
    );
    for_each_row(reader, selected, offset, max_rows, |row| {
        println!(
            "{}",
            row.iter().map(|v| csv_escape(v)).collect::<Vec<_>>().join(",")
        );
        Ok(())
    })
}

fn print_json_stream(
    reader: ParquetRecordBatchReader,
    header: &[String],
    selected: &[usize],
    offset: usize,
    max_rows: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    println!("[");
    // Delay printing each row by one, so the comma goes after every row
    // except the last one (valid JSON without trailing commas).
    let mut pending: Option<String> = None;
    for_each_row(reader, selected, offset, max_rows, |row| {
        let fields = header
            .iter()
            .zip(row)
            .map(|(h, v)| format!("{}: {}", json_key(h), json_value(v)))
            .collect::<Vec<_>>()
            .join(", ");
        let line = format!("  {{{fields}}}");
        if let Some(prev) = pending.replace(line) {
            println!("{prev},");
        }
        Ok(())
    })?;
    if let Some(last) = pending {
        println!("{last}");
    }
    println!("]");
    Ok(())
}

fn print_table(header: &[String], rows: &[Vec<String>]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(header.iter().map(|h| Cell::new(h)).collect::<Vec<_>>());
    for r in rows {
        table.add_row(r.iter().map(|v| Cell::new(v)).collect::<Vec<_>>());
    }
    println!("{table}");
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn json_key(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn json_value(v: &str) -> String {
    if v == "NULL" {
        return "null".to_string();
    }
    if v == "true" || v == "false" {
        return v.to_string();
    }
    if v.parse::<i64>().is_ok() || v.parse::<f64>().is_ok() {
        return v.to_string();
    }
    format!(
        "\"{}\"",
        v.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}
