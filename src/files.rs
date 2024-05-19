use anyhow::{anyhow, Result};
use qdrant_client::qdrant::{value::Kind, ScoredPoint};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct File {
    pub path: String,
    pub contents: String,
    pub sentences: Vec<String>,
}

impl File {
    fn new(path: String, contents: String) -> Self {
        Self {
            path,
            contents,
            sentences: Vec::new(),
        }
    }

    pub fn parse(&mut self) {
        let mut contents = Vec::new();
        let mut state = FileState::None;
        let mut sentence = String::new();

        for line in self.contents.lines() {
            match state {
                FileState::None => {
                    if line.starts_with("```") {
                        state = FileState::CodeBlock;
                        sentence = String::new();
                        sentence.push_str(line);
                        sentence.push('\n');
                    } else if line.starts_with("---") {
                        state = FileState::Comments;
                    } else if !line.starts_with('#') && !line.is_empty() {
                        state = FileState::Sentence;
                        sentence = String::new();
                        sentence.push_str(line);
                        sentence.push('\n');
                    }
                }
                FileState::CodeBlock => {
                    sentence.push_str(line);
                    if line.starts_with("```") {
                        contents.push(sentence);
                        sentence = String::new();
                        state = FileState::None;
                    }
                }
                FileState::Comments => {
                    if line.starts_with("---") {
                        state = FileState::None;
                    }
                }
                FileState::Sentence => {
                    if line.is_empty() {
                        state = FileState::None;
                        contents.push(sentence);
                        sentence = String::new();
                    } else {
                        sentence.push_str(line);
                        sentence.push('\n');
                    }
                }
            }
        }
        self.sentences = contents;
    }
}

pub fn load_files_from_dir(dir: PathBuf, ending: &str, prefix: &PathBuf) -> Result<Vec<File>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            let mut sub_files = load_files_from_dir(path, ending, prefix)?;
            files.append(&mut sub_files);
        } else if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.to_str().unwrap() == "md" {
                    let contents = fs::read_to_string(&path)?;
                    let path = Path::new(&path).strip_prefix(prefix)?.to_owned();

                    let path_as_str = format!("{}", path.display());

                    if path_as_str.to_lowercase().starts_with("templates") {
                        println!(
                            "File was skipped because it's in the Templates directory: {}",
                            path.display()
                        );
                        continue;
                    }
                    let key = path
                        .to_str()
                        .ok_or(anyhow!("Could not get string slice from path"))?;
                    let mut file = File::new(key.to_string(), contents);
                    file.parse();
                    files.push(file);
                }
            }
        }
    }
    Ok(files)
}
enum FileState {
    None,
    CodeBlock,
    Sentence,
    Comments,
}

pub trait Finder {
    fn find(&self, key: &str) -> Option<String>;
    fn get_contents(&self, result: &ScoredPoint) -> Option<String>;
}

impl Finder for Vec<File> {
    fn find(&self, key: &str) -> Option<String> {
        for file in self {
            if file.path == key {
                return Some(file.contents.clone());
            }
        }
        None
    }

    fn get_contents(&self, result: &ScoredPoint) -> Option<String> {
        let text = result.payload.get("id")?;
        let kind = text.kind.to_owned()?;
        if let Kind::StringValue(value) = kind {
            self.find(&value)
        } else {
            None
        }
    }
}
