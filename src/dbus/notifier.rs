//! D-Bus implementation for org.kde.StatusNotifierItem.
use crate::hyprland::{Hyprland, WindowInfo, Workspace};
use anyhow::Result;
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
    pub fn new(window_info: WindowInfo, exit_notify: Arc<Notify>, hyprland: Hyprland) -> Self {
        StatusNotifierItem {
            window_info,
            exit_notify,
            hyprland,
        }
    }

    /// A helper to wrap D-Bus actions. It executes the provided closure,
    /// logs any resulting error, and always sends an exit notification.
    fn handle_action(&self, action: impl FnOnce() -> Result<()>) {
        if let Err(e) = action() {
            eprintln!("[Error] Failed to execute hyprctl dispatch from notifier: {e}");
        }
        self.exit_notify.notify_one();
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
        self.handle_action(|| {
            let active_workspace = self.hyprland.exec::<Workspace>("activeworkspace")?;
            self.hyprland.dispatch(&format!(
                "movetoworkspace {},address:{}",
                active_workspace.id, self.window_info.address
            ))?;
            self.hyprland
                .dispatch(&format!("focuswindow address:{}", self.window_info.address))
        });
    }

    fn secondary_activate(&self, _x: i32, _y: i32) {
        self.handle_action(|| {
            self.hyprland
                .dispatch(&format!("closewindow address:{}", self.window_info.address))
        });
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

    #[derive(Default, Clone)]
    struct MockExecutor {
        dispatched_commands: Arc<Mutex<Vec<String>>>,
        // FIXED: This now correctly uses a Mutex for shared mutability.
        json_response: Arc<Mutex<String>>,
    }
    impl MockExecutor {
        // This method is no longer needed as we can lock and modify the mutex directly.
        // fn set_json_response(&mut self, json: &str) {
        //     self.json_response = json.to_string();
        // }
        fn dispatched_commands(&self) -> Vec<String> {
            self.dispatched_commands.lock().unwrap().clone()
        }
    }
    impl hyprland::HyprctlExecutor for MockExecutor {
        fn execute_json(&self, _command: &str) -> Result<Output, anyhow::Error> {
            Ok(Output {
                status: ExitStatus::from_raw(0),
                // FIXED: Lock the mutex to get the response.
                stdout: self.json_response.lock().unwrap().as_bytes().to_vec(),
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

    // Helper to create a standard StatusNotifierItem for tests.
    fn create_test_item(executor: Arc<MockExecutor>) -> (StatusNotifierItem, Arc<Notify>) {
        let notify = Arc::new(Notify::new());
        let window_info = WindowInfo {
            address: "0xNOTIFY_TEST".to_string(),
            class: "NotifierApp".to_string(),
            title: "Notifier Window".to_string(),
            workspace: Workspace { id: 1 },
        };
        let hyprland = Hyprland::new(executor as Arc<dyn hyprland::HyprctlExecutor>);
        let item = StatusNotifierItem::new(window_info, Arc::clone(&notify), hyprland);
        (item, notify)
    }

    // --- The Tests ---

    #[tokio::test]
    async fn test_activate_restores_and_focuses_window() {
        let mock_executor = Arc::new(MockExecutor::default());
        let (item, notify) = create_test_item(mock_executor.clone());
        // FIXED: Lock the mutex to set the JSON response for the test.
        mock_executor
            .json_response
            .lock()
            .unwrap()
            .push_str(r#"{"id": 7}"#);

        item.activate(0, 0);

        let dispatched = mock_executor.dispatched_commands();
        assert_eq!(dispatched.len(), 2);
        assert_eq!(dispatched[0], "movetoworkspace 7,address:0xNOTIFY_TEST");
        assert_eq!(dispatched[1], "focuswindow address:0xNOTIFY_TEST");

        assert!(timeout(Duration::from_millis(10), notify.notified())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_secondary_activate_closes_window() {
        let mock_executor = Arc::new(MockExecutor::default());
        let (item, notify) = create_test_item(mock_executor.clone());

        item.secondary_activate(0, 0);

        let dispatched = mock_executor.dispatched_commands();
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0], "closewindow address:0xNOTIFY_TEST");

        assert!(timeout(Duration::from_millis(10), notify.notified())
            .await
            .is_ok());
    }
}
