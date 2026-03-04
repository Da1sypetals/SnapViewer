use crate::{
    constants::ELEMENT_DB_FILENAME,
    utils::{get_spinner, memory_usage},
};
use duckdb::{types::ValueRef, Connection};
use std::path::Path;

fn value_ref_to_string(v: ValueRef<'_>) -> duckdb::Result<String> {
    Ok(match v {
        ValueRef::Text(b) => String::from_utf8_lossy(b).into_owned(),
        ValueRef::Int(n) => n.to_string(),
        ValueRef::TinyInt(n) => n.to_string(),
        ValueRef::SmallInt(n) => n.to_string(),
        ValueRef::BigInt(n) => n.to_string(),
        ValueRef::HugeInt(n) => n.to_string(),
        ValueRef::UInt(n) => n.to_string(),
        ValueRef::UTinyInt(n) => n.to_string(),
        ValueRef::USmallInt(n) => n.to_string(),
        ValueRef::UBigInt(n) => n.to_string(),
        ValueRef::Float(f) => f.to_string(),
        ValueRef::Double(f) => f.to_string(),
        ValueRef::Blob(b) => format!("<BLOB len={}>", b.len()),
        ValueRef::Null => String::from("NULL"),
        ValueRef::Boolean(b) => b.to_string(),
        _ => String::from("[UNSUPPORTED TYPE]"),
    })
}

#[derive(Debug)]
pub struct AllocationDatabase {
    pub conn: Connection,
}

impl AllocationDatabase {
    pub fn from_dir(dir: &Path) -> anyhow::Result<Self> {
        log::info!("Creating allocations database");
        println!(
            "Memory before connecting to database: {} MiB",
            memory_usage()
        );

        let parquet_path = dir.join(ELEMENT_DB_FILENAME);
        let parquet_path_str = parquet_path.to_str().unwrap();

        let conn = Connection::open_in_memory()?;
        conn.execute_batch(&format!(
            "CREATE VIEW allocs AS SELECT * FROM read_parquet('{}')",
            parquet_path_str
        ))?;

        Ok(Self { conn })
    }

    pub fn row_count(&self) -> anyhow::Result<usize> {
        let bar = get_spinner("Querying # of rows...")?;
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM allocs")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        bar.finish();
        Ok(count as usize)
    }

    pub fn get_callstack(&self, idx: usize) -> anyhow::Result<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT callstack FROM allocs WHERE idx = ?")?;
        let callstack: String = stmt.query_row(duckdb::params![idx as i64], |row| row.get(0))?;
        Ok(callstack)
    }

    pub fn execute(&self, command: &str) -> anyhow::Result<String> {
        log::info!("Executing SQL query");

        let mut stmt = self.conn.prepare(command)?;
        // query() executes the statement internally, so column metadata is valid immediately.
        let mut rows = stmt.query([])?;
        let num_cols = rows.as_ref().unwrap().column_count();
        let column_names: Vec<String> = rows
            .as_ref()
            .unwrap()
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let mut output_string = String::new();
        output_string.push_str("\n========== SQL Query Results ==========\n");

        let mut idx = 0usize;
        while let Some(row) = rows.next()? {
            let row_values = (0..num_cols)
                .map(|i| value_ref_to_string(row.get_ref(i)?))
                .collect::<Result<Vec<_>, _>>()?;

            output_string.push_str(&format!("\n\nRow {:>3}:\n", idx));
            output_string.push_str("+------------------------+------------------------+\n");

            let mut callstack_str = None;
            for (col_name, row_value) in column_names.iter().zip(row_values) {
                if col_name == "callstack" {
                    callstack_str = Some(format!("callstack:\n{}", row_value));
                } else {
                    output_string.push_str(&format!("| {:<22} | {:<22} |\n", col_name, row_value));
                }
            }

            output_string.push_str("+------------------------+------------------------+\n");

            if let Some(callstack_str) = callstack_str {
                output_string.push_str(&callstack_str);
            }

            idx += 1;
        }

        Ok(output_string)
    }
}
