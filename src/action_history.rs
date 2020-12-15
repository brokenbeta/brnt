use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Change {
    from: String,
    to: String,
}

pub struct ChangeList {
    list: Vec<Change>,
}

impl ChangeList {
    pub fn new() -> Self {
        Self { list: Vec::new() }
    }

    pub fn push(&mut self, from: &PathBuf, to: &PathBuf) {
        let from: String = String::from(from.to_str().unwrap());
        let to: String = String::from(to.to_str().unwrap());
        self.list.push(Change { from: from, to: to });
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActionHistory {
    changes: Vec<Vec<Change>>,
    file_name: String,
}

impl ActionHistory {
    fn from_history(file_name: &str) -> Option<Self> {
        if let Ok(content) = fs::read(file_name) {
            let json_data =
                String::from_utf8(content).expect("Failed to read utf8 from history file");
            let mut action_history: ActionHistory = serde_json::from_str(&json_data)
                .expect("Failed to deserialize to ActionHistory from file.");
            action_history.file_name = String::from(file_name);
            return Some(action_history);
        }
        None
    }

    pub fn new(file_name: &str) -> Self {
        if let Some(action_history) = ActionHistory::from_history(file_name) {
            return action_history;
        }
        Self {
            changes: Vec::new(),
            file_name: String::from(file_name),
        }
    }

    pub fn write(changes: ChangeList, file_name: &str) {
        let mut action_history = ActionHistory::new(file_name);
        action_history.changes.push(changes.list);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&action_history.file_name)
            .unwrap();
        let json_string = serde_json::to_string_pretty(&action_history)
            .expect("Failed to serialize ActionHistory.");
        file.write_all(&json_string.into_bytes())
            .expect("Failed to write serialized ActionHistory into json");
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serial_test::serial;

    const TEST_FILE_NAME: &str = ".history_test.json";

    #[test]
    #[serial]
    fn test_write_history_to_file() {
        fs::remove_file(TEST_FILE_NAME).expect("Failed to drop test history file.");
        let mut change_list = ChangeList::new();
        change_list.push(&PathBuf::from("from1.txt"), &PathBuf::from("to1.txt"));
        change_list.push(&PathBuf::from("from2.txt"), &PathBuf::from("to2.txt"));
        ActionHistory::write(change_list, TEST_FILE_NAME);
    }

    #[test]
    fn test_get_action_history_from_file() {
        test_write_history_to_file();

        let action_history = ActionHistory::new(TEST_FILE_NAME);
        println!("{:?}", action_history);
        assert_eq!(action_history.changes.len(), 1);
        assert_eq!(action_history.changes[0].len(), 2);
    }

    #[test]
    fn test_create_empty_history() {
        let action_history = ActionHistory::new(TEST_FILE_NAME);
        assert_eq!(action_history.changes.len(), 1);
    }
}
