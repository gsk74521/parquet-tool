# parquet-tool

一个用 Rust 编写的 Parquet 文件查看工具，编译为单个自包含的可执行二进制文件，无需任何运行时依赖。提供 Linux（静态 musl）与 Windows 两种版本。

## 功能

- **schema** — 查看 parquet 文件的 schema（字段名、物理类型、逻辑类型、是否可空、嵌套结构）
- **rowcount** — 查看 parquet 文件的行数
- **show** — 查看 parquet 文件的详细内容，支持表格 / CSV / JSON 三种输出格式、行数限制、起始偏移、列筛选
- **cat** — 查看 parquet 文件的完整内容（全部行）
- **head** — 查看 parquet 文件的开头前几行（类似 Linux `head` 命令）
- **compression** — 查看压缩统计（逐列压缩前后大小、压缩算法、压缩比）

## 使用方法

```
parquet-tool <COMMAND> [OPTIONS]

Commands:
  schema       打印文件的 schema（含行数、row group 数、创建者信息）
  rowcount     打印文件的总行数
  show         查看文件内容（可限制行数）
  cat          查看文件的完整内容（全部行）
  head         查看文件的开头前 N 行
  compression  查看压缩统计（逐列压缩比）
```

### 查看 schema

```
parquet-tool schema <file.parquet>
```

输出示例：

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

### 查看行数

```
parquet-tool rowcount <file.parquet>
# 输出：100
```

### 查看内容

```
# 表格输出（默认），显示前 20 行
parquet-tool show <file.parquet>

# 只显示前 5 行
parquet-tool show <file.parquet> --limit 5

# 跳过前 98 行，从第 99 行开始显示
parquet-tool show <file.parquet> --offset 98 --limit 5

# 只看指定列
parquet-tool show <file.parquet> --columns id,name,score --limit 5

# CSV 输出
parquet-tool show <file.parquet> --format csv --limit 5

# JSON 输出
parquet-tool show <file.parquet> --format json --limit 5

# 显示全部行（--limit 0 表示不限制）
parquet-tool show <file.parquet> --limit 0
```

### 参数说明（show）

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `-l, --limit <N>` | 最多显示 N 行，`0` 表示全部 | `20` |
| `-o, --offset <N>` | 跳过前 N 行再开始显示 | `0` |
| `-f, --format <fmt>` | 输出格式：`table` / `csv` / `json` | `table` |
| `-c, --columns <cols>` | 只显示指定列（逗号分隔），如 `id,name` | 全部列 |

### cat —— 查看完整内容（全部行）

```
# 表格输出全部行
parquet-tool cat <file.parquet>

# CSV / JSON 输出全部行
parquet-tool cat <file.parquet> --format csv
parquet-tool cat <file.parquet> --format json

# 只看指定列
parquet-tool cat <file.parquet> --columns id,name
```

`cat` 会输出文件中的全部行，等价于 `show --limit 0`。CSV / JSON 输出为流式，即使文件很大也不会把所有数据一次性载入内存。

### head —— 查看开头前几行

```
# 查看前 10 行（默认）
parquet-tool head <file.parquet>

# 查看前 3 行
parquet-tool head <file.parquet> -n 3
parquet-tool head <file.parquet> --lines 3

# CSV / JSON / 指定列
parquet-tool head <file.parquet> -n 5 --format csv
parquet-tool head <file.parquet> -n 5 --format json --columns id,name
```

### 参数说明（cat / head）

| 命令 | 参数 | 说明 | 默认值 |
|------|------|------|--------|
| `cat` | `-f, --format <fmt>` | 输出格式：`table` / `csv` / `json` | `table` |
| `cat` | `-c, --columns <cols>` | 只显示指定列（逗号分隔） | 全部列 |
| `head` | `-n, --lines <N>` | 查看开头前 N 行 | `10` |
| `head` | `-f, --format <fmt>` | 输出格式：`table` / `csv` / `json` | `table` |
| `head` | `-c, --columns <cols>` | 只显示指定列（逗号分隔） | 全部列 |

### compression —— 查看压缩统计

```
parquet-tool compression <file.parquet>
```

输出示例（每列一行 + 汇总两行）：

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

说明：
- `uncompressed` / `compressed` 为该列所有数据页压缩前 / 压缩后的字节数（来自文件 footer 元数据，**不实际解压数据**，速度很快）
- `ratio` = `uncompressed / compressed`，大于 1 表示有压缩效果；数据量极小或规律性差的列可能出现略小于 1 的值（压缩开销大于收益，属正常现象）
- 最后两行汇总：`TOTAL (data only)` 只统计列数据；`TOTAL (incl. footer)` 用整个文件大小计算，反映真实磁盘占用（含 footer、页头等开销）

**重要：`uncompressed` 是字典编码（dictionary/RLE 编码）之后、压缩算法之前的尺寸，不是原始逻辑数据大小。**

Parquet 有**两层压缩**：先做字典编码（第一层，通常是最主要的一层），再做压缩算法（第二层）。当数据高度重复时，字典编码已把整列折叠成极小尺寸，此时第二层（codec）几乎压不动、只剩每页固定开销，导致 `ratio` 可能 < 1——这**不代表**文件存储低效。判断真实存储效率请直接看整个文件大小（对比原始文本/逻辑大小）。

## 支持的数据类型

支持 Parquet 的全部物理类型和逻辑类型，包括：
- 基础类型：INT32/INT64/INT96、FLOAT/DOUBLE、BOOLEAN、BYTE_ARRAY、FIXED_LEN_BYTE_ARRAY
- 逻辑类型：String、Decimal、Date、Time、Timestamp（含 IANA 时区，如 UTC）、JSON、UUID 等
- 嵌套结构：Struct（group）、List（list/repeated）等
- 各种压缩编码：Snappy、Gzip、Brotli、LZ4、Zstd（读取侧全部支持）

NULL 值在表格/CSV 中显示为 `NULL`，在 JSON 中输出为 `null`。

## 构建

### Windows 版

需要 Rust 工具链（rustc/cargo 1.75+）。

```
cargo build --release
```

产物：`target/release/parquet-tool.exe`

注意：Windows 下若 gcc 编译 zstd C 源码时因临时目录权限失败，可先将 `TMP`/`TEMP` 环境变量指向一个可写目录再构建。

### Linux 版（静态 musl，交叉编译）

需要 rustup + [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild)（`pip install cargo-zigbuild`，自带 zig）。

```
rustup target add x86_64-unknown-linux-musl
cargo zigbuild --release --target x86_64-unknown-linux-musl
```

产物：`target/x86_64-unknown-linux-musl/release/parquet-tool`

该产物是纯静态链接的 ELF 可执行文件，在任何 x86_64 Linux 上直接运行：

```bash
chmod +x parquet-tool
./parquet-tool head file.parquet
```

## 测试数据

`examples/gen_sample.rs` 是测试数据生成器，可生成包含多种类型、嵌套结构和 NULL 的示例文件：

```
cargo run --release --example gen_sample -- sample.parquet 100
```

## 项目结构

- `src/main.rs` — 主程序
- `examples/gen_sample.rs` — 测试数据生成器
- `Cargo.toml` — 依赖配置（parquet 60 / arrow 60 / clap / comfy-table）

## 许可证

[MIT License](LICENSE) — 详见项目根目录的 LICENSE 文件。

## 发布到 GitHub Releases

项目已包含自动发布工作流 [.github/workflows/release.yml](.github/workflows/release.yml)，推 `v*` 标签时自动交叉编译 Linux（静态 musl）和 Windows 二进制、打包（tar.gz / zip）、生成 SHA256 校验和并创建 GitHub Release。

**首次发布（一次性）**：
```bash
# 1. 在 GitHub 创建空仓库（不要勾选 README/.gitignore 初始化）
# 2. 关联并推送
git remote add origin https://github.com/<你的用户名>/parquet-tool.git
git branch -M main
git add -A && git commit -m "Initial release"
git push -u origin main
```

**发布新版本（以后每次）**：
```bash
# 1. 更新 Cargo.toml 的 version（如 1.1.0）
# 2. 提交并打标签
git add -A && git commit -m "Release 1.1.0"
git tag v1.1.0
git push origin main --tags
# GitHub Actions 自动构建并生成 Release，附件含 tar.gz/zip/SHA256SUMS
```

**release 管理要点**：
- **版本号规范**：标签用 `v<版本>`（如 `v1.0.0`），与 Cargo.toml 的 `version` 保持一致
- **语义化版本**：`MAJOR.MINOR.PATCH`——破坏性改动升 MAJOR，新增功能升 MINOR，修 bug 升 PATCH
- **Release notes**：workflow 用 `generate_release_notes: true` 自动生成；也可发布后在 GitHub 页面手动补充中文说明
- **回滚**：保留历史 tag/release，发现问题可下载旧版本；代码回滚用 `git revert`
- **检查状态**：仓库页 Actions 标签页查看构建进度，失败时点进日志排查
