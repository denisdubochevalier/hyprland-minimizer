//! D-Bus implementation for org.kde.StatusNotifierItem.
use crate::hyprland::{Hyprland, WindowInfo, Workspace};
use std::sync::Arc;
use tokio::sync::Notify;
use zbus::dbus_interface;
use zbus::zvariant::ObjectPath;

pub struct StatusNotifierItem {
    window_info: WindowInfo,
    exit_notify: Arc<Notify>,
    hyprland: Hyprland,
}

// Type alias to simplify the complex return type of `tool_tip`.
type ToolTip = (String, Vec<(i32, i32, Vec<u8>)>, String, String);

impl StatusNotifierItem {
    /// Instantiates StatusNotifierItem
    pub fn new(window_info: WindowInfo, exit_notify: Arc<Notify>, hyprland: &Hyprland) -> Self {
        StatusNotifierItem {
            window_info,
            exit_notify,
            hyprland: hyprland.clone(),
        }
    }
}

#[dbus_interface(name = "org.kde.StatusNotifierItem")]
impl StatusNotifierItem {
    #[dbus_interface(property)]
    fn category(&self) -> &str {
        "ApplicationStatus"
    }
    #[dbus_interface(property)]
    fn id(&self) -> &str {
        &self.window_info.class
    }
    #[dbus_interface(property)]
    fn title(&self) -> &str {
        &self.window_info.title
    }
    #[dbus_interface(property)]
    fn status(&self) -> &str {
        "Active"
    }
    #[dbus_interface(property)]
    fn icon_name(&self) -> &str {
        &self.window_info.class
    }
    #[dbus_interface(property)]
    fn tool_tip(&self) -> ToolTip {
        (
            String::new(),
            Vec::new(),
            self.window_info.title.clone(),
            String::new(),
        )
    }
    #[dbus_interface(property)]
    fn item_is_menu(&self) -> bool {
        false
    }
    #[dbus_interface(property)]
    fn menu(&self) -> ObjectPath<'_> {
        ObjectPath::try_from("/Menu").unwrap()
    }

    fn activate(&self, _x: i32, _y: i32) {
        if let Ok(active_workspace) = self.hyprland.exec::<Workspace>("activeworkspace") {
            let _ = self
                .hyprland
                .dispatch(&format!(
                    "movetoworkspace {},address:{}",
                    active_workspace.id, self.window_info.address
                ))
                .and_then(|_| {
                    self.hyprland
                        .dispatch(&format!("focuswindow address:{}", self.window_info.address))
                });
        }
        self.exit_notify.notify_one();
    }

    fn secondary_activate(&self, _x: i32, _y: i32) {
        let _ = self
            .hyprland
            .dispatch(&format!("closewindow address:{}", self.window_info.address));
        self.exit_notify.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyprland;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::time::timeout;

    // --- Mocking Setup ---

    // A mock executor that records dispatched commands and provides mock JSON.
    #[derive(Default, Clone)]
    struct MockExecutor {
        dispatched_commands: Arc<Mutex<Vec<String>>>,
        json_response: String,
    }
    impl MockExecutor {
        fn new() -> Self {
            Default::default()
        }
        fn set_json_response(&mut self, json: &str) {
            self.json_response = json.to_string();
        }
        fn dispatched_commands(&self) -> Vec<String> {
            self.dispatched_commands.lock().unwrap().clone()
        }
    }
    impl hyprland::HyprctlExecutor for MockExecutor {
        fn execute_json(&self, _command: &str) -> Result<Output, anyhow::Error> {
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: self.json_response.as_bytes().to_vec(),
                stderr: vec![],
            })
        }
        fn execute_dispatch(&self, command: &str) -> Result<Output, anyhow::Error> {
            self.dispatched_commands
                .lock()
                .unwrap()
                .push(command.to_string());
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: vec![],
                stderr: vec![],
            })
        }
    }

    // Helper to swap the real executor with our mock for the duration of a test.
    fn with_mock_executor(mock: MockExecutor, test_fn: impl FnOnce()) {
        hyprland::EXECUTOR.with(|cell| {
            *cell.borrow_mut() = Box::new(mock);
        });
        test_fn();
        hyprland::EXECUTOR.with(|cell| {
            *cell.borrow_mut() = Box::new(hyprland::LiveExecutor);
        });
    }

    // Helper to create a standard StatusNotifierItem for tests.
    fn create_test_item() -> (StatusNotifierItem, Arc<Notify>) {
        let notify = Arc::new(Notify::new());
        let item = StatusNotifierItem {
            window_info: WindowInfo {
                address: "0xNOTIFY_TEST".to_string(),
                class: "NotifierApp".to_string(),
                title: "Notifier Window".to_string(),
                workspace: Workspace { id: 1 },
            },
            exit_notify: Arc::clone(&notify),
            hyprland: Hyprland::new(),
        };
        (item, notify)
    }

    // --- The Tests ---

    #[tokio::test]
    async fn test_activate_restores_and_focuses_window() {
        let (item, notify) = create_test_item();
        let mut mock_executor = MockExecutor::new();
        // Simulate `hyprctl activeworkspace` returning workspace 7
        mock_executor.set_json_response(r#"{"id": 7}"#);

        with_mock_executor(mock_executor.clone(), || {
            item.activate(0, 0);
        });

        // Assert that the correct commands were dispatched
        let dispatched = mock_executor.dispatched_commands();
        assert_eq!(dispatched.len(), 2);
        assert_eq!(dispatched[0], "movetoworkspace 7,address:0xNOTIFY_TEST");
        assert_eq!(dispatched[1], "focuswindow address:0xNOTIFY_TEST");

        // Assert that the exit signal was sent
        assert!(
            timeout(Duration::from_millis(10), notify.notified())
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_secondary_activate_closes_window() {
        let (item, notify) = create_test_item();
        let mock_executor = MockExecutor::new();

        with_mock_executor(mock_executor.clone(), || {
            item.secondary_activate(0, 0);
        });

        // Assert that the correct command was dispatched
        let dispatched = mock_executor.dispatched_commands();
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0], "closewindow address:0xNOTIFY_TEST");

        // Assert that the exit signal was sent
        assert!(
            timeout(Duration::from_millis(10), notify.notified())
                .await
                .is_ok()
        );
    }
}
