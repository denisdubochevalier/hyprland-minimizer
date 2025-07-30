//! D-Bus implementation for com.canonical.dbusmenu.
use crate::hyprland::{Hyprland, WindowInfo, Workspace};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Notify;
use zbus::dbus_interface;
use zbus::zvariant::Value;

pub struct DbusMenu {
    window_info: WindowInfo,
    exit_notify: Arc<Notify>,
    hyprland: Hyprland,
}

// Type alias to simplify the complex return type of `get_layout`.
// The values created are all owned, so we can use the 'static lifetime.
type MenuLayout<'a> = (u32, (i32, HashMap<String, Value<'a>>, Vec<Value<'a>>));

impl DbusMenu {
    /// Instantiates DbusMenu
    pub fn new(window_info: WindowInfo, exit_notify: Arc<Notify>, hyprland: &Hyprland) -> Self {
        DbusMenu {
            window_info,
            exit_notify,
            // FIXED: Clone the hyprland instance to take ownership.
            hyprland: hyprland.clone(),
        }
    }

    /// Handles the logic for opening the window on the currently active workspace.
    fn handle_open_on_active(&self) -> Result<()> {
        let active_workspace = self.hyprland.exec::<Workspace>("activeworkspace")?;
        self.hyprland.dispatch(&format!(
            "movetoworkspace {},address:{}",
            active_workspace.id, self.window_info.address
        ))?;
        self.hyprland
            .dispatch(&format!("focuswindow address:{}", self.window_info.address))
    }

    /// Handles the logic for opening the window on its original workspace.
    fn handle_open_on_original(&self) -> Result<()> {
        self.hyprland.dispatch(&format!(
            "movetoworkspace {},address:{}",
            self.window_info.workspace.id, self.window_info.address
        ))?;
        self.hyprland
            .dispatch(&format!("focuswindow address:{}", self.window_info.address))
    }

    /// Handles the logic for closing the window.
    fn handle_close(&self) -> Result<()> {
        self.hyprland
            .dispatch(&format!("closewindow address:{}", self.window_info.address))
    }
}

#[dbus_interface(name = "com.canonical.dbusmenu")]
impl DbusMenu {
    /// Returns the menu layout.
    fn get_layout(
        &self,
        _parent_id: i32,
        _recursion_depth: i32,
        _property_names: Vec<String>,
    ) -> MenuLayout<'static> {
        let mut open_props = HashMap::new();
        open_props.insert("type".to_string(), Value::from("standard"));
        open_props.insert(
            "label".to_string(),
            Value::from(format!("Open {}", self.window_info.title)),
        );
        let open_item = Value::from((1i32, open_props, Vec::<Value>::new()));

        let mut last_ws_props = HashMap::new();
        last_ws_props.insert("type".to_string(), Value::from("standard"));
        last_ws_props.insert(
            "label".to_string(),
            Value::from(format!(
                "Open on original workspace ({})",
                self.window_info.workspace.id
            )),
        );
        let last_ws_item = Value::from((2i32, last_ws_props, Vec::<Value>::new()));

        let mut close_props = HashMap::new();
        close_props.insert("type".to_string(), Value::from("standard"));
        close_props.insert(
            "label".to_string(),
            Value::from(format!("Close {}", self.window_info.title)),
        );
        let close_item = Value::from((3i32, close_props, Vec::<Value>::new()));

        let mut root_props = HashMap::new();
        root_props.insert("children-display".to_string(), Value::from("submenu"));
        let root_layout = (0i32, root_props, vec![open_item, last_ws_item, close_item]);
        (2u32, root_layout)
    }

    /// Returns the properties for a group of menu items.
    fn get_group_properties(
        &self,
        ids: Vec<i32>,
        _property_names: Vec<String>,
    ) -> Vec<(i32, HashMap<String, Value>)> {
        let mut result = Vec::new();
        for id in ids {
            let mut props = HashMap::new();
            let label = match id {
                1 => format!("Open {}", self.window_info.title),
                2 => format!(
                    "Open on original workspace ({})",
                    self.window_info.workspace.id
                ),
                3 => format!("Close {}", self.window_info.title),
                _ => continue,
            };
            props.insert("label".to_string(), Value::from(label));
            props.insert("enabled".to_string(), Value::from(true));
            props.insert("visible".to_string(), Value::from(true));
            props.insert("type".to_string(), Value::from("standard"));
            result.push((id, props));
        }
        result
    }

    /// Handles a batch of click events.
    fn event_group(&self, events: Vec<(i32, String, Value<'_>, u32)>) {
        for (id, event_id, data, timestamp) in events {
            self.event(id, &event_id, data, timestamp);
        }
    }

    /// Handles a single click event on a menu item.
    fn event(&self, id: i32, event_id: &str, _data: Value<'_>, _timestamp: u32) {
        if event_id != "clicked" {
            return;
        }

        let res = match id {
            1 => self.handle_open_on_active(),
            2 => self.handle_open_on_original(),
            3 => self.handle_close(),
            _ => return,
        };

        if let Err(e) = res {
            eprintln!("[Error] Failed to execute hyprctl dispatch from menu: {e}");
        }

        self.exit_notify.notify_one();
    }

    fn about_to_show_group(&self, _ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        (vec![], vec![])
    }

    fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    #[dbus_interface(property)]
    fn version(&self) -> u32 {
        3
    }

    #[dbus_interface(property)]
    fn text_direction(&self) -> &str {
        "ltr"
    }

    #[dbus_interface(property)]
    fn status(&self) -> &str {
        "normal"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyprland;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};
    use std::sync::Mutex;
    use std::time::Duration;
    use tokio::time::timeout;

    // --- Mocking Setup for hyprland calls ---

    #[derive(Default, Clone)]
    struct MockExecutor {
        dispatched_commands: Arc<Mutex<Vec<String>>>,
        json_responses: Arc<Mutex<Vec<String>>>,
    }
    impl MockExecutor {
        fn add_json_response(&self, json: &str) {
            self.json_responses.lock().unwrap().push(json.to_string());
        }
        fn dispatched_commands(&self) -> Vec<String> {
            self.dispatched_commands.lock().unwrap().clone()
        }
    }
    impl hyprland::HyprctlExecutor for MockExecutor {
        fn execute_json(&self, _command: &str) -> Result<Output> {
            let response = self
                .json_responses
                .lock()
                .unwrap()
                .pop()
                .unwrap_or_default();
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: response.as_bytes().to_vec(),
                stderr: vec![],
            })
        }
        fn execute_dispatch(&self, command: &str) -> Result<Output> {
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

    // Helper to create a standard DbusMenu for tests.
    fn create_test_menu(executor: Arc<MockExecutor>) -> (DbusMenu, Arc<Notify>) {
        let notify = Arc::new(Notify::new());
        let window_info = WindowInfo {
            address: "0xTEST".to_string(),
            class: "TestApp".to_string(),
            title: "Test Window".to_string(),
            workspace: Workspace { id: 1 },
        };
        let hyprland = Hyprland::new(executor as Arc<dyn hyprland::HyprctlExecutor>);
        let menu = DbusMenu::new(window_info, Arc::clone(&notify), &hyprland);
        (menu, notify)
    }

    // --- The Tests ---

    #[tokio::test]
    async fn test_event_click_option_1_open_on_active() {
        let mock_executor = Arc::new(MockExecutor::default());
        let (menu, notify) = create_test_menu(mock_executor.clone());
        mock_executor.add_json_response(r#"{"id": 5}"#);

        // FIXED: Call the event to trigger the action.
        menu.event(1, "clicked", Value::from(0), 0);

        // Assert that the correct commands were dispatched
        let dispatched = mock_executor.dispatched_commands();
        assert_eq!(dispatched.len(), 2);
        assert_eq!(dispatched[0], "movetoworkspace 5,address:0xTEST");
        assert_eq!(dispatched[1], "focuswindow address:0xTEST");

        // Assert that the exit signal was sent
        assert!(timeout(Duration::from_millis(10), notify.notified())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_event_click_option_2_open_on_original() {
        let mock_executor = Arc::new(MockExecutor::default());
        let (menu, notify) = create_test_menu(mock_executor.clone());

        // FIXED: Call the event to trigger the action.
        menu.event(2, "clicked", Value::from(0), 0);

        let dispatched = mock_executor.dispatched_commands();
        assert_eq!(dispatched.len(), 2);
        assert_eq!(dispatched[0], "movetoworkspace 1,address:0xTEST");
        assert_eq!(dispatched[1], "focuswindow address:0xTEST");
        assert!(timeout(Duration::from_millis(10), notify.notified())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_event_click_option_3_close_window() {
        let mock_executor = Arc::new(MockExecutor::default());
        let (menu, notify) = create_test_menu(mock_executor.clone());

        // FIXED: Call the event to trigger the action.
        menu.event(3, "clicked", Value::from(0), 0);

        let dispatched = mock_executor.dispatched_commands();
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0], "closewindow address:0xTEST");
        assert!(timeout(Duration::from_millis(10), notify.notified())
            .await
            .is_ok());
    }
}
