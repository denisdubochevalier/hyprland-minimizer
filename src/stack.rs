//! Stack management for minimized windows using a file.
use anyhow::{Context, Result, bail};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Constructs a user-specific temporary filepath using the $USER environment variable.
fn get_stack_file_path() -> Result<PathBuf> {
    match env::var("USER") {
        Ok(username) => {
            if username.is_empty() {
                bail!("The USER environment variable was empty.");
            }
            let file_path = format!("/tmp/hypr-minimizer-stack-{}", username);
            Ok(PathBuf::from(file_path))
        }
        Err(_) => bail!("Could not find the USER environment variable."),
    }
}

// Represents the stack file.
#[derive(Debug, Clone)]
pub struct Stack {
    path: PathBuf,
}

impl Stack {
    #[cfg(test)]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Stack { path: path.into() }
    }

    /// Creates a Stack instance by determining the user-specific default path.
    /// This can fail if the user cannot be determined from the environment.
    pub fn at_default_path() -> Result<Self> {
        let path = get_stack_file_path()?;

        Ok(Stack { path })
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
    use std::env;
    use tempfile::NamedTempFile;

    #[test]
    fn at_default_path_success_when_user_is_set() {
        // --- Setup ---
        // Set a temporary environment variable for this test.
        let test_user = "testuser";
        unsafe {
            env::set_var("USER", test_user);
        }

        // --- Execute ---
        // Call the function we want to test.
        let result = Stack::at_default_path();

        // --- Assert ---
        // Ensure the function returned an Ok variant.
        assert!(result.is_ok());

        // Unwrap the successful result to inspect the Stack instance.
        let stack = result.unwrap();
        let expected_path = PathBuf::from(format!("/tmp/hypr-minimizer-stack-{}", test_user));

        // Check if the path inside the struct is what we expect.
        assert_eq!(stack.path, expected_path);

        // --- Teardown ---
        // It's good practice to clean up the environment variable,
        // although it won't affect other tests in this case.
        unsafe {
            env::remove_var("USER");
        }
    }

    #[test]
    fn at_default_path_fails_when_user_is_not_set() {
        // --- Setup ---
        // Ensure the environment variable is not set.
        unsafe {
            env::remove_var("USER");
        }

        // --- Execute ---
        let result = Stack::at_default_path();

        // --- Assert ---
        // Ensure the function returned an Err variant.
        assert!(result.is_err());

        // Optionally, check for the specific error message.
        let error_message = result.unwrap_err().to_string();
        assert_eq!(
            error_message,
            "Could not find the USER environment variable."
        );
    }

    #[test]
    fn test_stack_operations() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let stack = Stack::new(temp_file.path());

        assert!(stack.pop()?.is_none());

        stack.push("addr1")?;
        stack.push("addr2")?;
        stack.push("addr3")?;

        assert_eq!(stack.pop()?.unwrap(), "addr3");
        assert_eq!(stack.pop()?.unwrap(), "addr2");

        stack.push("addr2-restored")?;
        stack.push("addr3-restored")?;
        // Stack is now: [addr1, addr2-restored, addr3-restored]
        stack.remove("addr2-restored")?;
        // Stack should be: [addr1, addr3-restored]

        assert_eq!(stack.pop()?.unwrap(), "addr3-restored");
        assert_eq!(stack.pop()?.unwrap(), "addr1");
        assert!(stack.pop()?.is_none());

        Ok(())
    }
}
