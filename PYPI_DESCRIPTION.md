<p align="center">
  <h1 align="center">parquet-tool</h1>
  <p align="center">Inspect Apache Parquet files from the command line — schema, row count, contents, compression statistics.<br/>命令行查看 Apache Parquet 文件：schema、行数、内容、压缩统计。</p>
</p>

<p align="center">
  <a href="#english">English</a> · <a href="#中文">中文</a>
</p>

---

## English

parquet-tool is a command-line tool written in **Rust** for inspecting Apache Parquet files. It compiles into a single self-contained static executable with **zero runtime dependencies** and is distributed as both a native binary and a `pip`-installable package.

### Features

| Command       | Description |
|---------------|-------------|
| `schema`      | Print the schema (field names, physical/logical types, nullability, nesting) |
| `rowcount`    | Print the total number of rows |
| `show`        | Print contents as table / CSV / JSON, with row limit, offset and column filter |
| `cat`         | Print the entire contents (all rows) |
| `head`        | Print the first N rows (like Linux `head`) |
| `distinct`    | Count distinct (unique non-null) values per column, plus NULL counts |
| `compression` | Print per-column and per-file compression statistics (sizes, codec, ratio) |

### Install

```bash
pip install parquet-tool
```

### Quick start

```bash
# Print the schema
parquet-tool schema data.parquet

# Print the row count
parquet-tool rowcount data.parquet

# Print the first 5 rows
parquet-tool head data.parquet -n 5

# Print all rows as JSON
parquet-tool cat data.parquet --format json

# Count distinct values per column
parquet-tool distinct data.parquet

# Print compression statistics
parquet-tool compression data.parquet
```

### Supported data types

All Parquet physical and logical types: INT32/INT64/INT96, FLOAT/DOUBLE, BOOLEAN, BYTE_ARRAY, FIXED_LEN_BYTE_ARRAY; logical String, Decimal, Date, Time, Timestamp (with IANA timezones), JSON, UUID; nested Struct/List; codecs Snappy, Gzip, Brotli, LZ4, Zstd.

### Platforms

| Platform | Wheel | Notes |
|----------|-------|-------|
| Linux x86_64 | `musllinux_1_2_x86_64` | statically linked musl — runs on any Linux |
| Windows x86_64 | `win_amd64` | native MSVC build |

### License

[MIT](https://github.com/gsk74521/parquet-tool/blob/main/LICENSE)

---

## 中文

parquet-tool 是一个用 **Rust** 编写的 Apache Parquet 文件查看工具，编译为单个自包含的静态可执行文件，**零运行时依赖**。提供原生二进制与 `pip` 安装包两种分发方式。

### 功能

| 命令          | 说明 |
|---------------|------|
| `schema`      | 查看 schema（字段名、物理/逻辑类型、是否可空、嵌套结构） |
| `rowcount`    | 查看总行数 |
| `show`        | 以表格 / CSV / JSON 查看内容，支持行数限制、起始偏移、列筛选 |
| `cat`         | 查看完整内容（全部行） |
| `head`        | 查看开头前 N 行（类似 Linux `head`） |
| `distinct`    | 查看每一列的 distinct（唯一非空）值数量，以及 NULL 数量 |
| `compression` | 查看逐列及文件级压缩统计（压缩前后大小、算法、压缩比） |

### 安装

```bash
pip install parquet-tool
```

### 快速开始

```bash
# 查看 schema
parquet-tool schema data.parquet

# 查看行数
parquet-tool rowcount data.parquet

# 查看前 5 行
parquet-tool head data.parquet -n 5

# 以 JSON 输出全部内容
parquet-tool cat data.parquet --format json

# 查看每一列的 distinct 值数量
parquet-tool distinct data.parquet

# 查看压缩统计
parquet-tool compression data.parquet
```

### 支持的数据类型

全部 Parquet 物理与逻辑类型：INT32/INT64/INT96、FLOAT/DOUBLE、BOOLEAN、BYTE_ARRAY、FIXED_LEN_BYTE_ARRAY；逻辑类型 String、Decimal、Date、Time、Timestamp（支持 IANA 时区）、JSON、UUID；嵌套 Struct/List；压缩算法 Snappy、Gzip、Brotli、LZ4、Zstd。

### 支持平台

| 平台 | Wheel | 说明 |
|------|-------|------|
| Linux x86_64 | `musllinux_1_2_x86_64` | 静态 musl 链接 —— 任何 Linux 均可运行 |
| Windows x86_64 | `win_amd64` | 原生 MSVC 构建 |

### 许可证

[MIT](https://github.com/gsk74521/parquet-tool/blob/main/LICENSE)
