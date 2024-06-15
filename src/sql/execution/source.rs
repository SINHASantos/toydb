use super::super::engine::Transaction;
use super::super::types::{Column, Expression, Row, Value};
use super::QueryIterator;
use crate::error::Result;

use std::collections::HashSet;

/// A table scan executor
pub struct Scan {
    table: String,
    filter: Option<Expression>,
}

impl Scan {
    pub fn new(table: String, filter: Option<Expression>) -> Self {
        Self { table, filter }
    }

    pub fn execute(self, txn: &mut impl Transaction) -> Result<QueryIterator> {
        let table = txn.must_read_table(&self.table)?;
        Ok(QueryIterator {
            columns: table.columns.iter().map(|c| Column { name: Some(c.name.clone()) }).collect(),
            rows: Box::new(txn.scan(&table.name, self.filter)?),
        })
    }
}

/// A primary key lookup executor
pub struct KeyLookup {
    table: String,
    keys: Vec<Value>,
}

impl KeyLookup {
    pub fn new(table: String, keys: Vec<Value>) -> Self {
        Self { table, keys }
    }

    pub fn execute(self, txn: &mut impl Transaction) -> Result<QueryIterator> {
        let table = txn.must_read_table(&self.table)?;

        // FIXME Is there a way to pass the txn into an iterator closure instead?
        let rows = self
            .keys
            .into_iter()
            .filter_map(|key| txn.read(&table.name, &key).transpose())
            .collect::<Result<Vec<Row>>>()?;

        Ok(QueryIterator {
            columns: table.columns.iter().map(|c| Column { name: Some(c.name.clone()) }).collect(),
            rows: Box::new(rows.into_iter().map(Ok)),
        })
    }
}

/// An index value lookup executor
pub struct IndexLookup {
    table: String,
    column: String,
    values: Vec<Value>,
}

impl IndexLookup {
    pub fn new(table: String, column: String, values: Vec<Value>) -> Self {
        Self { table, column, values }
    }

    pub fn execute(self, txn: &mut impl Transaction) -> Result<QueryIterator> {
        let table = txn.must_read_table(&self.table)?;

        let mut pks: HashSet<Value> = HashSet::new();
        for value in self.values {
            pks.extend(txn.read_index(&self.table, &self.column, &value)?);
        }

        // FIXME Is there a way to pass the txn into an iterator closure instead?
        let rows = pks
            .into_iter()
            .filter_map(|pk| txn.read(&table.name, &pk).transpose())
            .collect::<Result<Vec<Row>>>()?;

        Ok(QueryIterator {
            columns: table.columns.iter().map(|c| Column { name: Some(c.name.clone()) }).collect(),
            rows: Box::new(rows.into_iter().map(Ok)),
        })
    }
}

/// An executor that produces a single empty row
pub struct Nothing;

impl Nothing {
    pub fn execute(self) -> QueryIterator {
        QueryIterator { columns: Vec::new(), rows: Box::new(std::iter::once(Ok(Row::new()))) }
    }
}
