mod copy_path;
mod create_directory;
mod delete_path;
mod edit_file;
mod fetch;
mod find_path;
mod grep;
mod list_directory;
mod move_path;
mod mutation;
mod read_file;
mod terminal;
mod workspace;
mod write_file;

pub use copy_path::CopyPathTool;
pub use create_directory::CreateDirectoryTool;
pub use delete_path::DeletePathTool;
pub use edit_file::EditFileTool;
pub use fetch::FetchTool;
pub use find_path::FindPathTool;
pub use grep::GrepTool;
pub use list_directory::ListDirectoryTool;
pub use move_path::MovePathTool;
pub use read_file::ReadFileTool;
pub use terminal::TerminalTool;
pub use write_file::WriteFileTool;

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::{Value, json};

    use super::*;
    use crate::Tool;

    struct TestWorkspace(PathBuf);

    impl TestWorkspace {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("golden-tools-{}", uuid::Uuid::now_v7()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    async fn call(tool: impl Tool, args: Value) -> Value {
        serde_json::from_str(&tool.call(args).await.unwrap()).unwrap()
    }

    #[tokio::test]
    async fn find_and_grep_skip_ignored_files() {
        let workspace = TestWorkspace::new();
        fs::write(workspace.0.join(".gitignore"), "ignored.rs\n").unwrap();
        fs::write(workspace.0.join("ignored.rs"), "fn example() {}\n").unwrap();
        fs::write(workspace.0.join("main.rs"), "\nfn example() {}\n").unwrap();
        let found = call(
            FindPathTool::new(&workspace.0).unwrap(),
            json!({ "pattern": "**/*.rs" }),
        )
        .await;
        assert_eq!(found["paths"], json!(["main.rs"]));

        let grep = GrepTool::new(&workspace.0).unwrap();
        let args = json!({ "pattern": "^fn\\s+example" });
        let matches = call(&grep, args.clone()).await;
        assert_eq!(matches["matches"].as_array().unwrap().len(), 1);
        assert_eq!(matches["matches"][0]["path"], "main.rs");
        assert_eq!(matches["matches"][0]["line"], 2);

        let matches = call(grep, json!({ "pattern": "example", "path": "main.rs" })).await;
        assert_eq!(matches["matches"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn read_file_respects_line_ranges_and_workspace_paths() {
        let workspace = TestWorkspace::new();
        fs::write(workspace.0.join("sample.txt"), "first\r\n第二行\nthird").unwrap();
        let tool = ReadFileTool::new(&workspace.0).unwrap();
        let result = call(
            &tool,
            json!({ "path": "sample.txt", "start_line": 2, "end_line": 2 }),
        )
        .await;
        assert_eq!(result["content"], "第二行\n");
        assert_eq!(result["end_line"], 2);
        assert_eq!(result["truncated"], false);
        assert!(tool.call(json!({ "path": "../sample.txt" })).await.is_err());

        fs::write(workspace.0.join("sample.txt"), "line\n".repeat(201)).unwrap();
        let result = call(tool, json!({ "path": "sample.txt" })).await;
        assert_eq!(result["end_line"], 200);
        assert_eq!(result["truncated"], true);
    }

    #[tokio::test]
    async fn workspace_mutations_preserve_existing_entries_and_require_unique_edits() {
        let workspace = TestWorkspace::new();
        let root = &workspace.0;
        let directory = CreateDirectoryTool::new(root).unwrap();
        let writer = WriteFileTool::new(root).unwrap();
        let editor = EditFileTool::new(root).unwrap();
        let copier = CopyPathTool::new(root).unwrap();
        let mover = MovePathTool::new(root).unwrap();
        let deleter = DeletePathTool::new(root).unwrap();

        call(directory, json!({ "path": "src/nested" })).await;
        let path = "src/nested/sample.txt";
        call(
            &writer,
            json!({ "path": path, "content": "hello hello world" }),
        )
        .await;
        assert!(
            writer
                .call(json!({ "path": path, "content": "overwritten" }))
                .await
                .is_err()
        );
        assert!(
            editor
                .call(json!({ "path": path, "old_text": "hello", "new_text": "bye" }))
                .await
                .is_err()
        );
        assert_eq!(
            fs::read_to_string(root.join(path)).unwrap(),
            "hello hello world"
        );
        call(
            editor,
            json!({ "path": path, "old_text": "world", "new_text": "世界" }),
        )
        .await;
        call(
            writer,
            json!({ "path": "occupied.txt", "content": "keep me" }),
        )
        .await;
        assert!(
            copier
                .call(json!({ "source": path, "destination": "occupied.txt" }))
                .await
                .is_err()
        );
        assert!(
            mover
                .call(json!({ "source": path, "destination": "occupied.txt" }))
                .await
                .is_err()
        );
        assert!(
            copier
                .call(json!({ "source": "src", "destination": "src/copy" }))
                .await
                .is_err()
        );
        call(copier, json!({ "source": "src", "destination": "backup" })).await;
        call(
            mover,
            json!({ "source": "backup/nested/sample.txt", "destination": "saved.txt" }),
        )
        .await;
        assert_eq!(
            fs::read_to_string(root.join("saved.txt")).unwrap(),
            "hello hello 世界"
        );
        assert!(!root.join("backup/nested/sample.txt").exists());

        assert!(deleter.call(json!({ "path": "src" })).await.is_err());
        assert!(
            deleter
                .call(json!({ "path": ".", "recursive": true }))
                .await
                .is_err()
        );
        call(deleter, json!({ "path": "src", "recursive": true })).await;
        assert!(!root.join("src").exists());
        assert_eq!(
            fs::read_to_string(root.join("occupied.txt")).unwrap(),
            "keep me"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn workspace_mutations_reject_symlinks_and_clean_up_failed_directory_copies() {
        let workspace = TestWorkspace::new();
        let outside = TestWorkspace::new();
        let source = workspace.0.join("src");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("file.txt"), "data").unwrap();
        std::os::unix::fs::symlink(&outside.0, source.join("link")).unwrap();

        let writer = WriteFileTool::new(&workspace.0).unwrap();
        assert!(
            writer
                .call(json!({ "path": "src/link/file.txt", "content": "data" }))
                .await
                .is_err()
        );
        assert!(!outside.0.join("file.txt").exists());

        let copier = CopyPathTool::new(&workspace.0).unwrap();
        assert!(
            copier
                .call(json!({ "source": "src", "destination": "copy" }))
                .await
                .is_err()
        );
        assert!(!workspace.0.join("copy").exists());
    }
}
