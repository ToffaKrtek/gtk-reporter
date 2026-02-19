use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use crate::error::ReporterError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Row {
    pub id: u32,
    pub text: String,
    pub status: Status,
}

impl Row {
    pub fn new(id: u32, text: String) -> Self {
        Self {
            id,
            text,
            status: Status::Working,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Status {
    Working,
    Testing,
    Ready,
}

impl Status {
    pub fn to_str(&self) -> &str {
        match self {
            Self::Working => "В работе",
            Self::Testing => "Отдал в тестирование",
            Self::Ready => "Готово",
            _ => "В работе",
        }
    }
}

const PATH_STATE_FILE: &str = "~/.gtk-reporter/gtk-reporter.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub max_id: u32,
    pub rows: HashMap<String, Vec<Row>>,
    pub cur_date: String,
}

impl State {
    pub fn load() -> Result<Self, ReporterError> {
        let mut file = File::open(PATH_STATE_FILE)?;
        let mut json_string = String::new();
        file.read_to_string(&mut json_string)?;
        let mut s: State = serde_json::from_str(&json_string)?;
        s.cur_date = chrono::Local::now().format("%Y-%m-%d").to_string();
        Ok(s)
    }
    pub fn save(&self) -> Result<(), ReporterError> {
        let mut file = File::create(PATH_STATE_FILE)?;
        let json_string = serde_json::to_string(self)?;
        file.write_all(json_string.as_bytes())?;
        Ok(())
    }

    pub fn add_row(&mut self, text: String) -> Result<u32, ReporterError> {
        self.max_id += 1;
        self.rows
            .entry(self.cur_date.clone())
            .or_default()
            .push(Row::new(self.max_id, text));
        Ok(self.max_id)
    }

    pub fn edit_row(&mut self, key: String, id: u32, text: String) -> Result<(), ReporterError> {
        let rows = self.rows.get_mut(&key).ok_or(ReporterError::DateNotFound)?;
        let row = rows
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or(ReporterError::RowNotFound)?;
        row.text = text;
        Ok(())
    }
    pub fn update_row_status(
        &mut self,
        key: String,
        id: u32,
        status: Status,
    ) -> Result<(), ReporterError> {
        let rows = self.rows.get_mut(&key).ok_or(ReporterError::DateNotFound)?;
        let row = rows
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or(ReporterError::RowNotFound)?;
        row.status = status;
        Ok(())
    }

    pub fn delete_row(&mut self, key: String, id: u32) -> Result<(), ReporterError> {
        let rows = self.rows.get_mut(&key).ok_or(ReporterError::DateNotFound)?;
        let initial_len = rows.len();
        rows.retain(|r| r.id != id);
        if rows.len() == initial_len {
            return Err(ReporterError::RowNotFound);
        }
        if rows.is_empty() {
            self.rows.remove(&key);
        }
        Ok(())
    }

    pub fn move_row(&mut self, key: String, id: u32, new_key: String) -> Result<(), ReporterError> {
        let source_rows = self.rows.get_mut(&key).ok_or(ReporterError::DateNotFound)?;
        let row_index = source_rows
            .iter()
            .position(|r| r.id == id)
            .ok_or(ReporterError::RowNotFound)?;
        let row = source_rows.remove(row_index);
        if source_rows.is_empty() {
            self.rows.remove(&key);
        }
        self.rows.entry(new_key).or_default().push(row);
        Ok(())
    }
}
