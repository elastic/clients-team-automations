use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use anyhow::Context;
use comfy_table::Table;
use trustfall::{execute_query, FieldValue};

use crate::adapter::SkillsAdapter;
use crate::config::Config;
use crate::convert;
use crate::data;
use crate::schema;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum QueryFormat {
    Table,
    Json,
    Csv,
}

pub fn run_query(
    query_str: Option<&str>,
    args_json: &str,
    format: &QueryFormat,
    skills_dir: &Path,
    config: &Config,
) -> anyhow::Result<()> {
    let query = match query_str {
        Some(q) => q.to_string(),
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("failed to read query from stdin")?;
            buf
        }
    };

    if query.trim().is_empty() {
        anyhow::bail!("no query provided (pass --query or pipe via stdin)");
    }

    let args: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(args_json).context("invalid JSON arguments")?;

    let trustfall_args: BTreeMap<String, FieldValue> = args
        .into_iter()
        .map(|(k, v)| (k, convert::json_to_field_value(&v)))
        .collect();

    let repo_root = std::env::current_dir().context("cannot get current directory")?;
    let skills_data = data::load_skills_data(skills_dir, &repo_root, config, None);
    let adapter = SkillsAdapter::new(skills_data);
    let schema = schema::schema();

    let results = execute_query(&schema, Arc::new(adapter), &query, trustfall_args)
        .context("query execution failed")?;

    let rows: Vec<BTreeMap<Arc<str>, FieldValue>> = results.collect();

    match format {
        QueryFormat::Table => print_table(&rows),
        QueryFormat::Json => print_json(&rows),
        QueryFormat::Csv => print_csv(&rows),
    }

    Ok(())
}

fn field_value_to_string(v: &FieldValue) -> String {
    match v {
        FieldValue::String(s) => s.to_string(),
        FieldValue::Int64(i) => i.to_string(),
        FieldValue::Uint64(u) => u.to_string(),
        FieldValue::Float64(f) => f.to_string(),
        FieldValue::Boolean(b) => b.to_string(),
        FieldValue::Null => String::new(),
        FieldValue::List(l) => {
            let items: Vec<String> = l.iter().map(field_value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        other => format!("{other:?}"),
    }
}

fn collect_headers(rows: &[BTreeMap<Arc<str>, FieldValue>]) -> Vec<String> {
    if rows.is_empty() {
        return Vec::new();
    }
    rows[0].keys().map(|k| k.to_string()).collect()
}

fn print_table(rows: &[BTreeMap<Arc<str>, FieldValue>]) {
    if rows.is_empty() {
        println!("(no results)");
        return;
    }

    let headers = collect_headers(rows);
    let string_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            headers
                .iter()
                .map(|h| {
                    let key: Arc<str> = Arc::from(h.as_str());
                    row.get(&key).map(field_value_to_string).unwrap_or_default()
                })
                .collect()
        })
        .collect();

    let mut table = Table::new();
    table.set_header(headers.clone());
    table.add_rows(string_rows);
    println!("{table}");
}

fn print_json(rows: &[BTreeMap<Arc<str>, FieldValue>]) {
    let json_rows: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let obj: serde_json::Map<String, serde_json::Value> = row
                .iter()
                .map(|(k, v)| (k.to_string(), convert::field_value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&json_rows).expect("failed to serialize results")
    );
}

fn print_csv(rows: &[BTreeMap<Arc<str>, FieldValue>]) {
    if rows.is_empty() {
        return;
    }

    let headers = collect_headers(rows);
    let mut wtr = csv::WriterBuilder::new()
        .from_writer(std::io::stdout());

    let header_record: Vec<&str> = headers.iter().map(String::as_str).collect();
    let _ = wtr.write_record(&header_record);

    for row in rows {
        let cells: Vec<String> = headers
            .iter()
            .map(|h| {
                let key: Arc<str> = Arc::from(h.as_str());
                row.get(&key).map(field_value_to_string).unwrap_or_default()
            })
            .collect();
        let record: Vec<&str> = cells.iter().map(String::as_str).collect();
        let _ = wtr.write_record(&record);
    }

    let _ = wtr.flush();
}
