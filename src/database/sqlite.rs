use crate::{allocation::Allocation, database::data_structure::format_callstack};
use indicatif::ProgressIterator;
use rusqlite::Connection;

pub const CREATE_SQL: &str = "CREATE TABLE allocs (
    idx INTEGER PRIMARY KEY,
    size INTEGER,
    start_time INTEGER,
    end_time INTEGER,
    callstack TEXT
);";
pub const INSERT_SQL: &str =
    "INSERT INTO allocs (idx, size, start_time, end_time, callstack) VALUES (?, ?, ?, ?, ?)";

pub struct AllocationDatabase {
    pub conn: Connection,
}

impl AllocationDatabase {
    pub fn from_allocations(allocations: &[Allocation]) -> anyhow::Result<Self> {
        log::info!("Creating allocations database");

        let mut conn = Connection::open_in_memory()?;

        conn.execute(CREATE_SQL, ())?;

        log::info!("Inserting rows into allocations table");
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(INSERT_SQL)?;
            for (index, alloc) in allocations.iter().enumerate().progress() {
                stmt.execute((
                    &index,
                    &alloc.size,
                    &alloc.timesteps[0],
                    alloc.timesteps.last().unwrap(),
                    format_callstack(&alloc.callstack),
                ))?;
            }
        }
        tx.commit()?;

        Ok(Self { conn })
    }

    pub fn execute(&self, command: &str) -> anyhow::Result<String> {
        log::info!("Executing SQL query");

        let mut stmt = self.conn.prepare(command)?;
        let num_cols = stmt.column_count();
        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

        let mut output_string = String::new();
        let rows_iter = stmt.query_map([], |row| {
            let mut row_values = Vec::new();
            for i in 0..num_cols {
                let value_str = row
                    .get_ref(i)?
                    .as_str()
                    .map(|s| s.to_string()) // if is text
                    .unwrap_or_else(|_| {
                        // If not text, try to represent it as a string
                        match row.get_ref(i) {
                            Ok(rusqlite::types::ValueRef::Integer(i)) => i.to_string(),
                            Ok(rusqlite::types::ValueRef::Real(f)) => f.to_string(),
                            Ok(rusqlite::types::ValueRef::Blob(b)) => {
                                format!("<BLOB len={}>", b.len())
                            }
                            Ok(rusqlite::types::ValueRef::Null) => String::from("NULL"),
                            _ => String::from("[UNSUPPORTED TYPE]"),
                        }
                    });
                row_values.push(value_str);
            }
            Ok(row_values)
        })?;

        // log::info!("Merging results");
        // output_string.push_str("\n===== SQL Query Results =====\n");
        // for (idx, row_result) in rows_iter.enumerate() {
        //     let row_values = row_result?;
        //     output_string.push_str(&format!("\n===== Row {} =====\n", idx));
        //     for (col_name, row_value) in column_names.iter().zip(row_values) {
        //         output_string.push_str(&format!("column [{}] : {}\n", col_name, row_value));
        //     }
        // }

        output_string.push_str("\n========== SQL Query Results ==========\n");

        for (idx, row_result) in rows_iter.enumerate() {
            let row_values = row_result?;

            output_string.push_str(&format!("\n\nRow {:>3}:\n", idx));
            output_string.push_str("┌────────────────────────┬────────────────────────┐\n");

            let mut callstack_str = None;
            for (col_name, row_value) in column_names.iter().zip(row_values) {
                if col_name == "callstack" {
                    callstack_str = Some(format!("callstack:\n{}", row_value));
                } else {
                    output_string.push_str(&format!("│ {:<22} │ {:<22} │\n", col_name, row_value));
                }
            }

            output_string.push_str("└────────────────────────┴────────────────────────┘\n");

            if let Some(callstack_str) = callstack_str {
                output_string.push_str(&callstack_str);
            }
        }

        Ok(output_string)
    }
}
