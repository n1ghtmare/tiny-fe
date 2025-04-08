use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    time::SystemTime,
};

pub const DEFAULT_INDEX_FILE_NAME: &str = ".tiny-fe";

#[derive(Debug)]
pub struct DirectoryIndexEntry {
    /// Unix timestamp of the last access
    last_accessed: u64,
    /// Combined score based on frequence and recency
    rank: f64,
}

impl DirectoryIndexEntry {
    fn new() -> Self {
        DirectoryIndexEntry {
            last_accessed: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            rank: 0.0,
        }
    }

    fn update(&mut self) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.rank = (self.rank * 0.99) + 1.0;
        self.last_accessed = now;
    }

    fn frecent_score(&self) -> f64 {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let dx = now - self.last_accessed;

        // Calculate the frecent score, this was taken from rupa/z: https://github.com/rupa/z
        10000.0 * self.rank * (3.75 / ((0.0001 * dx as f64 + 1.0) + 0.25))
    }
}

pub struct DirectoryIndex {
    path: PathBuf,
    data: HashMap<PathBuf, DirectoryIndexEntry>,
}

impl DirectoryIndex {
    /// Reads the index from disk, if it doesn't exist, creates a new one
    pub fn load_from_disk(path: PathBuf) -> anyhow::Result<Self> {
        let file = if path.exists() {
            // Open the file if it exists
            File::open(&path)?
        } else {
            // Create the file if it doesn't exist
            File::create_new(&path)?
        };

        let reader = BufReader::new(file);
        let mut data = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split('|').collect();

            if parts.len() != 3 {
                // Skip malformed lines
                continue;
            }

            let path = PathBuf::from(parts[0]);
            let rank: f64 = parts[1].parse().unwrap_or(0.0);
            let last_accessed: u64 = parts[2].parse().unwrap_or(0);

            let entry = DirectoryIndexEntry {
                last_accessed,
                rank,
            };
            data.insert(path.clone(), entry);
        }

        Ok(DirectoryIndex { path, data })
    }

    pub fn save_to_disk(&self) -> anyhow::Result<()> {
        // Save the index to disk
        let file = File::create(self.path.clone())?;
        let mut writer = BufWriter::new(file);

        for (path, entry) in &self.data {
            writeln!(
                writer,
                "{}|{}|{}",
                path.display(),
                entry.rank,
                entry.last_accessed
            )?;
        }

        Ok(())
    }

    pub fn push_entry(&mut self, path: &PathBuf) {
        if let Some(entry) = self.data.get_mut(path) {
            // Entry exists, update it (to update the score and last accessed time)
            entry.update();
        } else {
            let entry = DirectoryIndexEntry::new();
            self.data.insert(path.clone(), entry);
        }
    }

    pub fn find_top_ranked(&self, query: &str) -> Option<PathBuf> {
        // This algorithm is largely taken from rupa/z: https://github.com/rupa/z
        let mut matches = Vec::new();
        let query_lower = query.to_lowercase();

        for (path, stats) in &self.data {
            let path_str = path.to_string_lossy();
            let frecent_score = stats.frecent_score();

            if path_str.contains(query) {
                // Higher priority: case-sensitive match.
                matches.push((path.clone(), frecent_score, 0));
            } else if path_str.to_lowercase().contains(&query_lower) {
                // Lower priority: case-insensitive match.
                matches.push((path.clone(), frecent_score, 1));
            }
        }

        if matches.is_empty() {
            return None;
        }

        // Look for a candidate that is an ancestor of every match.
        if let Some((ancestor, _, _)) = matches.iter().find(|(candidate, _, _)| {
            matches
                .iter()
                .all(|(other, _, _)| other.starts_with(candidate))
        }) {
            return Some(ancestor.clone());
        }

        // Fallback: sort by match priority, frecent score (high to low), and then by fewer path components.
        matches.sort_by(|a, b| {
            a.2.cmp(&b.2)
                .then(b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal))
                .then(a.0.components().count().cmp(&b.0.components().count()))
        });

        matches.first().map(|(path, _, _)| path.clone())
    }
}
