use core::fmt;

use crate::config::Config;

use super::utils::{self, query_wrapper, Header};

const TABLE_NAME: &str = "record";

pub struct ApartmentRecrod {
    pub id: Header<String>,
    pub datetime: Header<String>,
    pub price: Header<String>,
    pub url: Header<String>,
    pub brief: Header<String>,
}

impl ApartmentRecrod {
    pub fn new() -> Self {
        Self {
            id: Header::new(String::new(), "id"),
            datetime: Header::new(String::new(), "datetime"),
            price: Header::new(String::new(), "price"),
            url: Header::new(String::new(), "url"),
            brief: Header::new(String::new(), "brief"),
        }
    }
    fn create_table(&self) -> Result<(), rusqlite::Error> {
        let conn = utils::open(Config::database_location())?;
        let query = query_wrapper(format!(
            // id(text), price(text), url(text), brief(text)
            "CREATE TABLE IF NOT EXISTS {} (
            {}  TEXT NOT NULL,
            {}  TEXT NOT NULL UNIQUE,
            {}  TEXT NOT NULL,
            {}  TEXT NOT NULL,
            {}  TEXT NOT NULL
            )",
            TABLE_NAME,
            self.datetime.name,
            self.id.name,
            self.price.name,
            self.url.name,
            self.brief.name,
        ));

        conn.execute(&query, ())?;

        Ok(())
    }

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let mut a = ApartmentRecrod::new();
        a.id.value = row.get(0)?;
        a.price.value = row.get(1)?;
        a.url.value = row.get(2)?;
        a.brief.value = row.get(3)?;
        Ok(a)
    }

    pub fn select_one_by<T: fmt::Display>(h: &Header<T>) -> Result<Self, rusqlite::Error> {
        let conn = utils::open(Config::database_location())?;
        let query = query_wrapper(format!(
            "SELECT * FROM {} WHERE {}='{}' ORDER BY rowid DESC LIMIT 1",
            TABLE_NAME, h.name, h.value,
        ));

        let mut stmt = conn.prepare(&query)?;
        let mut record_iter = stmt.query_map([], |row| Self::from_row(row))?;
        record_iter
            .next()
            .unwrap_or_else(|| Err(rusqlite::Error::QueryReturnedNoRows))
    }

    pub fn insert(&self) -> Result<Self, rusqlite::Error> {
        let conn = utils::open(Config::database_location())?;
        self.create_table()?;
        let query = query_wrapper(format!(
            "INSERT INTO {} ({}, {},{},{},{})
            VALUES (?, ?, ?, ?, ?)",
            TABLE_NAME,
            self.datetime.name,
            self.id.name,
            self.price.name,
            self.url.name,
            self.brief.name
        ));
        conn.execute(
            &query,
            (
                &self.datetime.value,
                &self.id.value,
                &self.price.value,
                &self.url.value,
                &self.brief.value,
            ),
        )?;

        let query = query_wrapper(format!(
            "SELECT * FROM {} ORDER BY {} DESC LIMIT 1",
            TABLE_NAME, self.id.name
        ));

        let mut stmt = conn.prepare(&query)?;
        let mut record_iter = stmt.query_map([], |row| Self::from_row(row))?;

        // Ok(User::new())
        let record = record_iter.next().unwrap();
        record
    }

    // pub fn store(&self) -> Result<(), rusqlite::Error> {}
}
