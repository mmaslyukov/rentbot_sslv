use rusqlite::{Connection, Result};

pub enum DatabaseSource {
    Memory,
    File(String),
}

pub fn open(src: DatabaseSource) -> Result<Connection> {
    match src {
        DatabaseSource::Memory => Ok(Connection::open_in_memory()?),
        DatabaseSource::File(path) => Ok(Connection::open(path.as_str())?),
    }
}

#[derive(Debug, PartialEq)]
pub struct Header<T> {
    pub name: &'static str,
    pub value: T,
}

impl<T> Header<T> {
    pub fn new(value: T, name: &'static str) -> Self {
        Self { value, name }
    }
}

#[inline]
pub fn query_wrapper(query: String) -> String {
    let mut query_final = query.replace("\n", " ");
    while let Some(_) = query_final.find("  ") {
        query_final = query_final.replace("  ", " ");
    }
    log::debug!("#SQL: [{}]", query_final);
    query_final
}
