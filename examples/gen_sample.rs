//! Generate a sample parquet file with a variety of types, including
//! nested structs, lists, nulls, timestamps, etc.
//! Usage: cargo run --release --example gen_sample -- <output.parquet> [row_count]

use std::sync::Arc;

use arrow::array::{
    ArrayRef, BooleanArray, Float64Array, Int32Array, Int64Array, ListArray, StringArray,
    StructArray, TimestampMicrosecondArray,
};
use arrow::datatypes::{DataType, Field, Fields, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let out = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "sample.parquet".to_string());
    let n: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);

    // --- Build columns ---
    let id: ArrayRef = Arc::new(Int64Array::from_iter_values(0..n as i64));

    let names: Vec<Option<String>> = (0..n).map(|i| Some(format!("user_{i:05}"))).collect();
    let name: ArrayRef = Arc::new(StringArray::from_iter(names.iter().map(|x| x.as_deref())));

    // float with some nulls
    let score: ArrayRef = Arc::new(Float64Array::from_iter((0..n).map(|i| {
        if i % 7 == 0 {
            None
        } else {
            Some(i as f64 * 1.5)
        }
    })));

    // boolean
    let active: ArrayRef = Arc::new(BooleanArray::from_iter((0..n).map(|i| Some(i % 2 == 0))));

    // timestamp (microseconds, UTC)
    let mut ts_builder = TimestampMicrosecondArray::builder(n).with_timezone_opt(Some("UTC"));
    for i in 0..n {
        ts_builder.append_value(1_600_000_000_000_000 + i as i64 * 1_000_000);
    }
    let ts: ArrayRef = Arc::new(ts_builder.finish());

    // nested struct column: { city: string, zip: int32 }
    let cities: Vec<Option<String>> = (0..n).map(|i| Some(format!("city_{}", i % 5))).collect();
    let city: ArrayRef = Arc::new(StringArray::from_iter(cities.iter().map(|x| x.as_deref())));
    let zip: ArrayRef = Arc::new(Int32Array::from_iter_values(
        (0..n).map(|i| 10000 + (i % 100) as i32),
    ));
    let address = Arc::new(StructArray::from(vec![
        (Arc::new(Field::new("city", DataType::Utf8, true)), city),
        (Arc::new(Field::new("zip", DataType::Int32, false)), zip),
    ]));

    // list column: tags (variable length per row: row i has (i % 4) tags)
    let mut offsets: Vec<i32> = Vec::with_capacity(n + 1);
    let mut acc: i32 = 0;
    offsets.push(0);
    for i in 0..n {
        acc += (i % 4) as i32;
        offsets.push(acc);
    }
    let tags_values: Vec<String> = (0..acc).map(|i| format!("tag_{}", i % 8)).collect();
    let tags_offsets = arrow::buffer::OffsetBuffer::new(arrow::buffer::ScalarBuffer::from(offsets));
    let tags_values_arr: ArrayRef = Arc::new(StringArray::from_iter(
        tags_values.iter().map(|x| Some(x.as_str())),
    ));
    let tags = Arc::new(ListArray::new(
        Arc::new(Field::new("item", DataType::Utf8, true)),
        tags_offsets,
        tags_values_arr,
        None,
    ));

    // --- Schema ---
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, true),
        Field::new("score", DataType::Float64, true),
        Field::new("active", DataType::Boolean, true),
        Field::new(
            "ts",
            DataType::Timestamp(TimeUnit::Microsecond, Some(Arc::from("UTC"))),
            true,
        ),
        Field::new(
            "address",
            DataType::Struct(Fields::from(vec![
                Field::new("city", DataType::Utf8, true),
                Field::new("zip", DataType::Int32, false),
            ])),
            true,
        ),
        Field::new(
            "tags",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        ),
    ]));

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![id, name, score, active, ts, address, tags],
    )?;

    let file = std::fs::File::create(&out)?;
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
    writer.write(&batch)?;
    writer.close()?;

    println!("wrote {out} with {n} rows");
    Ok(())
}
