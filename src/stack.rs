//! Stack management for minimized windows using a file.
use anyhow::{Context, Result};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const STACK_FILE_PATH: &str = "/tmp/hypr-minimizer-stack";

// NEW: A struct to represent the stack file.
pub struct Stack {
    path: PathBuf,
}

impl Stack {
    /// Creates a new Stack instance pointing to a specific path. (Useful for tests)
    #[cfg(test)]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Stack { path: path.into() }
    }

    /// Creates a Stack instance pointing to the default application path.
    pub fn at_default_path() -> Self {
        Stack {
            path: PathBuf::from(STACK_FILE_PATH),
        }
    }

    /// Pushes a new address onto the stack file.
    pub fn push(&self, address: &str) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .context("Failed to open stack file for appending")?;
        writeln!(file, "{address}").context("Failed to append address to stack file")
    }

    /// Removes a specific address from anywhere in the stack file.
    pub fn remove(&self, address: &str) -> Result<()> {
        if !self.path.exists() {
            return Ok(());
        }
        let stack = read_stack(&self.path)?;
        let new_stack: Vec<String> = stack.into_iter().filter(|a| a.trim() != address).collect();
        write_stack(&self.path, &new_stack)
    }

    /// Pops the last address from the stack file.
    pub fn pop(&self) -> Result<Option<String>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let mut stack = read_stack(&self.path)?;
        let last = stack.pop();
        if last.is_some() {
            write_stack(&self.path, &stack)?;
        }
        Ok(last)
    }
}

// These helpers are now private to the module.
fn read_stack(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path).context("Failed to open stack file for reading")?;
    let reader = BufReader::new(file);
    reader
        .lines()
        .collect::<Result<_, _>>()
        .context("Failed to read lines from stack file")
}

fn write_stack(path: &Path, stack: &[String]) -> Result<()> {
    let mut file = File::create(path).context("Failed to open stack file for writing")?;
    for address in stack {
        writeln!(file, "{address}").context("Failed to write address to stack file")?;
    }
    Ok(())
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_stack_operations() -> Result<()> {
        // Create a temporary file that is automatically deleted.
        let temp_file = NamedTempFile::new()?;
        let stack = Stack::new(temp_file.path());

        // 1. Stack should be empty initially
        assert!(stack.pop()?.is_none());

        // 2. Push items
        stack.push("addr1")?;
        stack.push("addr2")?;
        stack.push("addr3")?;

        // 3. Pop items in LIFO order
        assert_eq!(stack.pop()?.unwrap(), "addr3");
        assert_eq!(stack.pop()?.unwrap(), "addr2");

        // 4. Remove an item from the middle
        stack.push("addr2-restored")?;
        stack.push("addr3-restored")?;
        // Stack is now: [addr1, addr2-restored, addr3-restored]
        stack.remove("addr2-restored")?;
        // Stack should be: [addr1, addr3-restored]

        // 5. Verify final state
        assert_eq!(stack.pop()?.unwrap(), "addr3-restored");
        assert_eq!(stack.pop()?.unwrap(), "addr1");
        assert!(stack.pop()?.is_none());

        Ok(())
    }
}
