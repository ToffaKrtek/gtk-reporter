use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::ReporterError;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Status {
    Working,
    Testing,
    Ready,
    Open,
}

impl Status {
    pub fn to_str(&self) -> &str {
        match self {
            Self::Working => "В работе",
            Self::Testing => "Передал в тестирование",
            Self::Ready => "Готово",
            Self::Open => "Открыто",
        }
    }
    pub fn to_string(&self) -> String {
        match self {
            Self::Working => "В работе".to_string(),
            Self::Testing => "Передал в тестирование".to_string(),
            Self::Ready => "Готово".to_string(),
            Self::Open => "Открыто".to_string(),
        }
    }

    pub fn all() -> [Status; 4] {
        [Status::Working, Status::Testing, Status::Ready, Self::Open]
    }
}

const PATH_STATE_FILE: &str = "~/.gtk-reporter/gtk-reporter.json";

fn get_state_file_path() -> PathBuf {
    let path = PATH_STATE_FILE.replace('~', &dirs::home_dir().unwrap().to_string_lossy());
    PathBuf::from(path)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub max_id: u32,
    pub rows: HashMap<String, Vec<Row>>,
    pub cur_date: String,
}

impl State {
    pub fn load() -> Result<Self, ReporterError> {
        let path = get_state_file_path();
        let mut file = File::open(&path)?;
        let mut json_string = String::new();
        file.read_to_string(&mut json_string)?;
        let mut s: State = serde_json::from_str(&json_string)?;
        s.cur_date = chrono::Local::now().format("%Y-%m-%d").to_string();
        Ok(s)
    }

    pub fn new() -> Self {
        Self {
            max_id: 0,
            rows: HashMap::new(),
            cur_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
        }
    }

    pub fn save(&self) -> Result<(), ReporterError> {
        let path = get_state_file_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&path)?;
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

    pub fn get_rows_for_date(&self, date: &str) -> Vec<Row> {
        self.rows.get(date).cloned().unwrap_or_default()
    }

    pub fn get_all_dates(&self) -> Vec<String> {
        let mut dates: Vec<String> = self.rows.keys().cloned().collect();
        dates.sort_by(|a, b| b.cmp(a));
        dates
    }

    pub fn get_row(&self, date: &str, id: u32) -> Option<Row> {
        self.rows
            .get(date)
            .and_then(|rows| rows.iter().find(|r| r.id == id))
            .cloned()
    }

    pub fn generate_report(&self, date: &str) -> String {
        let rows = self.get_rows_for_date(date);
        if rows.is_empty() {
            return format!("Отчет {}\n\nНет задач за эту дату.", date);
        }

        let mut report = format!("Отчет {}\n\n", date);

        for status in Status::all() {
            let status_rows: Vec<&Row> = rows.iter().filter(|r| r.status == status).collect();
            if !status_rows.is_empty() {
                report.push_str(&format!("=== {} ===\n", status.to_str()));
                for row in status_rows {
                    report.push_str(&format!("• {}\n", row.text));
                }
                report.push('\n');
            }
        }

        report
    }
}
