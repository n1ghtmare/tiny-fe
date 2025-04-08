use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

use tiny_fe::index::DirectoryIndex;

#[test]
fn directory_index_find_top_ranked_returns_correct_result() {
    let temp_dir = tempfile::tempdir().unwrap();
    let index_file_path = temp_dir.path().join(".tiny-fe");

    let mut file = File::create(&index_file_path).unwrap();

    let mock_data = vec![
        ("/home/user/documents/", 100, 100),
        ("/home/user/pictures/file2.txt", 20, 100),
        ("/home/user/projects/image1.jpg", 30, 100),
        ("/home/user/documents/test/", 40, 100),
    ];

    for line in mock_data {
        writeln!(file, "{}|{}|{}\n", line.0, line.1, line.2).unwrap();
    }

    let directory_index = DirectoryIndex::load_from_disk(index_file_path.clone()).unwrap();
    let result = directory_index.find_top_ranked("documents");

    assert_eq!(result, Some("/home/user/documents/".into()));
}

#[test]
fn directory_index_find_top_ranked_returns_none_for_no_match() {
    let temp_dir = tempfile::tempdir().unwrap();
    let index_file_path = temp_dir.path().join(".tiny-fe");

    let mut file = File::create(&index_file_path).unwrap();

    let mock_data = vec![
        ("/home/user/documents/", 100, 100),
        ("/home/user/pictures/file2.txt", 20, 100),
        ("/home/user/projects/image1.jpg", 30, 100),
        ("/home/user/documents/test/", 40, 100),
    ];

    for line in mock_data {
        writeln!(file, "{}|{}|{}\n", line.0, line.1, line.2).unwrap();
    }

    let directory_index = DirectoryIndex::load_from_disk(index_file_path.clone()).unwrap();
    let result = directory_index.find_top_ranked("nonexistent");

    assert_eq!(result, None);
}

#[test]
fn directory_index_find_top_ranked_returns_none_for_empty_index() {
    let temp_dir = tempfile::tempdir().unwrap();
    let index_file_path = temp_dir.path().join(".tiny-fe");

    let directory_index = DirectoryIndex::load_from_disk(index_file_path.clone()).unwrap();
    let result = directory_index.find_top_ranked("nonexistent");

    assert_eq!(result, None);
}

#[test]
fn directory_index_push_creates_a_new_entry() {
    let temp_dir = tempfile::tempdir().unwrap();
    let index_file_path = temp_dir.path().join(".tiny-fe");

    let mut directory_index = DirectoryIndex::load_from_disk(index_file_path.clone()).unwrap();

    let path = PathBuf::from("/home/user/documents/");
    directory_index.push_entry(&path);

    // Save the index to disk
    directory_index.save_to_disk().unwrap();

    // Check if the index file was created
    assert!(index_file_path.exists());

    // Check if the entry was added to the index
    let file = File::open(&index_file_path).unwrap();
    let reader = BufReader::new(file);

    let lines: Vec<String> = reader.lines().map(|line| line.unwrap()).collect();
    let line = &lines[0];

    let parts: Vec<&str> = line.split('|').collect();

    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "/home/user/documents/");
    assert_eq!(parts[1], "0");
}

#[test]
fn directory_index_push_multiple_times_updates_entry_rank() {
    let temp_dir = tempfile::tempdir().unwrap();
    let index_file_path = temp_dir.path().join(".tiny-fe");

    let mut directory_index = DirectoryIndex::load_from_disk(index_file_path.clone()).unwrap();

    let path = PathBuf::from("/home/user/documents/");
    directory_index.push_entry(&path);
    directory_index.push_entry(&path);
    directory_index.push_entry(&path);

    // Save the index to disk
    directory_index.save_to_disk().unwrap();

    // Check if the index file was created
    assert!(index_file_path.exists());

    // Check if the entry was added to the index
    let file = File::open(&index_file_path).unwrap();
    let reader = BufReader::new(file);

    let lines: Vec<String> = reader.lines().map(|line| line.unwrap()).collect();

    // Ensure that we still have only one line in the index file
    assert_eq!(lines.len(), 1);

    let line = &lines[0];

    let parts: Vec<&str> = line.split('|').collect();

    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "/home/user/documents/");
    assert_eq!(parts[1], "1.99");
}

#[test]
fn directory_index_find_top_ranked_returns_common_parent() {
    let temp_dir = tempfile::tempdir().unwrap();
    let index_file_path = temp_dir.path().join(".tiny-fe");

    let mut file = File::create(&index_file_path).unwrap();

    // Insert two related entries: the common parent and a subdirectory.
    // The common parent should be returned when queried.
    let mock_data = vec![
        ("/home/user/common-parent", 100, 100),
        ("/home/user/common-parent/test", 150, 100),
    ];

    for (path, rank, last_accessed) in mock_data {
        writeln!(file, "{}|{}|{}", path, rank, last_accessed).unwrap();
    }

    let directory_index = DirectoryIndex::load_from_disk(index_file_path.clone()).unwrap();
    let result = directory_index.find_top_ranked("common-parent");

    // Expect to get the common parent path rather than the subdirectory.
    assert_eq!(result, Some("/home/user/common-parent".into()));

    let result = directory_index.find_top_ranked("test");
    assert_eq!(result, Some("/home/user/common-parent/test".into()));
}
