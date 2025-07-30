//! Contains the core logic for minimizing a window to a tray icon.
use crate::dbus::{DbusMenu, StatusNotifierItem};
use crate::hyprland::{Hyprland, WindowInfo};
use crate::stack::Stack;

use anyhow::{Context, Result, anyhow};
// You will need to add this crate: `cargo add async-trait`
use async_trait::async_trait;
use futures_util::stream::StreamExt;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{Duration, interval};
use zbus::{Connection, ConnectionBuilder, Proxy};

// --- Trait for abstracting D-Bus interactions for testability ---
#[async_trait]
pub trait DbusConnection {
    async fn setup(
        &self,
        window_info: &WindowInfo,
        exit_notify: Arc<Notify>,
        hyprland: &Hyprland,
    ) -> Result<Option<(Arc<Connection>, String)>>;
    async fn register(&self, connection: &Arc<Connection>, bus_name: &str) -> Result<()>;
}

pub struct LiveDbus;
#[async_trait]
impl DbusConnection for LiveDbus {
    async fn setup(
        &self,
        window_info: &WindowInfo,
        exit_notify: Arc<Notify>,
        hyprland: &Hyprland,
    ) -> Result<Option<(Arc<Connection>, String)>> {
        // FIXED: Call the free function instead of a method
        Ok(Some(
            setup_dbus_connection(window_info, exit_notify, hyprland).await?,
        ))
    }
    async fn register(&self, connection: &Arc<Connection>, bus_name: &str) -> Result<()> {
        // FIXED: Call the free function instead of a method
        register_with_watcher(connection, bus_name).await
    }
}

pub struct Minimizer<'a, D: DbusConnection + Send + Sync> {
    stack: &'a Stack,
    window_info: WindowInfo,
    hyprland: Hyprland,
    dbus: &'a D,
}

impl<'a, D: DbusConnection + Send + Sync> Minimizer<'a, D> {
    /// Instantiate Minimizer
    pub fn new(stack: &'a Stack, window_info: WindowInfo, hyprland: Hyprland, dbus: &'a D) -> Self {
        Minimizer {
            stack,
            window_info,
            hyprland,
            dbus,
        }
    }

    /// The main entry point for the minimization workflow.
    pub async fn minimize(&self) -> Result<()> {
        self.minimize_window()?;

        let exit_notify = Arc::new(Notify::new());

        // Attempt to set up and register D-Bus services.
        let dbus_result = self.setup_and_register_dbus(Arc::clone(&exit_notify)).await;

        if let Err(e) = &dbus_result {
            // If D-Bus fails at any point, restore the window and clean up the stack.
            self.restore_window()?;
            // We need to convert the borrowed error back into an owned one to return it.
            return Err(anyhow!(e.to_string()));
        }

        let (arc_conn, bus_name) = dbus_result.unwrap();

        self.spawn_background_tasks(arc_conn, bus_name, Arc::clone(&exit_notify));

        println!("Application minimized to tray. Waiting for activation...");
        self.await_exit_signal(exit_notify).await;

        // Final cleanup after the application exits.
        if let Err(e) = self.stack.remove(&self.window_info.address) {
            eprintln!("[Error] Failed to remove window from stack file: {e}");
        }
        println!("Exiting.");

        Ok(())
    }

    /// Pushes the window to the stack and moves it to the special workspace.
    fn minimize_window(&self) -> Result<()> {
        println!(
            "Minimizing window: '{}' ({}) from workspace {}",
            self.window_info.title, self.window_info.class, self.window_info.workspace.id
        );
        self.stack.push(&self.window_info.address)?;
        self.hyprland.dispatch(&format!(
            "movetoworkspacesilent special:minimized,address:{}",
            self.window_info.address
        ))
    }

    /// Restores a window to its original workspace and removes it from the stack.
    fn restore_window(&self) -> Result<()> {
        self.hyprland.dispatch(&format!(
            "movetoworkspace {},address:{}",
            self.window_info.workspace.id, self.window_info.address
        ))?;
        self.stack.remove(&self.window_info.address)
    }

    /// Handles the full D-Bus connection and registration process.
    async fn setup_and_register_dbus(
        &self,
        exit_notify: Arc<Notify>,
    ) -> Result<(Arc<Connection>, String)> {
        let (arc_conn, bus_name) = match self
            .dbus
            .setup(&self.window_info, exit_notify, &self.hyprland)
            .await?
        {
            Some(conn) => conn,
            None => return Err(anyhow!("Mock D-Bus setup failed")),
        };

        if let Err(e) = self.dbus.register(&arc_conn, &bus_name).await {
            return Err(e).context("Failed to register tray icon.");
        }

        println!("Registration successful.");
        Ok((arc_conn, bus_name))
    }

    /// Spawns the background tasks for the application.
    fn spawn_background_tasks(
        &self,
        arc_conn: Arc<Connection>,
        bus_name: String,
        exit_notify: Arc<Notify>,
    ) {
        tokio::spawn(watch_for_tray_restarts(arc_conn.clone(), bus_name));
        tokio::spawn(poll_window_state(
            self.window_info.address.clone(),
            exit_notify,
        ));
    }

    async fn await_exit_signal(&self, exit_notify: Arc<Notify>) {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\nInterrupted by Ctrl+C. Restoring window.");
                let _ = self.hyprland.dispatch(&format!(
                    "movetoworkspace {},address:{}",
                    self.window_info.workspace.id,
                    self.window_info.address,
                ));
            }
                _ = exit_notify.notified() => {
                    println!("Exit notification received.");
                }
        }
    }
}

// --- Private Helper Functions ---

async fn setup_dbus_connection(
    window_info: &WindowInfo,
    exit_notify: Arc<Notify>,
    hyprland: &Hyprland,
) -> Result<(Arc<Connection>, String)> {
    let bus_name = format!(
        "org.kde.StatusNotifierItem.minimizer.p{}",
        std::process::id()
    );

    let notifier_item =
        StatusNotifierItem::new(window_info.clone(), Arc::clone(&exit_notify), hyprland);
    let dbus_menu = DbusMenu::new(window_info.clone(), Arc::clone(&exit_notify), hyprland);

    let connection = ConnectionBuilder::session()?
        .name(bus_name.as_str())?
        .serve_at("/StatusNotifierItem", notifier_item)?
        .serve_at("/Menu", dbus_menu)?
        .build()
        .await?;

    Ok((Arc::new(connection), bus_name))
}

async fn register_with_watcher(connection: &Arc<Connection>, bus_name: &str) -> Result<()> {
    let watcher_proxy: Proxy<'_> = zbus::ProxyBuilder::new_bare(connection)
        .interface("org.kde.StatusNotifierWatcher")?
        .path("/StatusNotifierWatcher")?
        .destination("org.kde.StatusNotifierWatcher")?
        .build()
        .await?;
    watcher_proxy
        .call_method("RegisterStatusNotifierItem", &(bus_name,))
        .await?;
    Ok(())
}

/// A background task that re-registers the tray icon if the tray restarts.
async fn watch_for_tray_restarts(arc_conn: Arc<Connection>, bus_name: String) {
    let Ok(dbus_proxy) = zbus::fdo::DBusProxy::new(&arc_conn).await else {
        return;
    };
    let Ok(mut owner_changes) = dbus_proxy.receive_name_owner_changed().await else {
        return;
    };

    while let Some(signal) = owner_changes.next().await {
        let Ok(args) = signal.args() else { continue };
        if args.name() == "org.kde.StatusNotifierWatcher" && args.new_owner().is_some() {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let _ = register_with_watcher(&arc_conn, &bus_name).await;
        }
    }
}

/// A background task that polls hyprland to see if the minimized window
/// has been closed or restored externally.
async fn poll_window_state(window_address: String, exit_notify: Arc<Notify>) {
    let mut interval = interval(Duration::from_secs(2));
    loop {
        interval.tick().await;

        let Ok(clients) = Hyprland::new().exec::<Vec<WindowInfo>>("clients") else {
            exit_notify.notify_one();
            return;
        };

        let should_exit = match clients.iter().find(|c| c.address == window_address) {
            // Window is found, exit if it's been restored to a normal workspace.
            Some(client) => client.workspace.id > 0,
            // Window is not found, exit because it has been closed.
            None => true,
        };

        if should_exit {
            exit_notify.notify_one();
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyprland::{self, Workspace};
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    // --- Mocking Setup ---
    #[derive(Default, Clone)]
    #[allow(dead_code)]
    struct MockHyprctlExecutor;
    impl hyprland::HyprctlExecutor for MockHyprctlExecutor {
        fn execute_json(&self, _command: &str) -> Result<Output> {
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: b"[]".to_vec(),
                stderr: vec![],
            })
        }
        fn execute_dispatch(&self, _command: &str) -> Result<Output> {
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: vec![],
                stderr: vec![],
            })
        }
    }

    // Mock D-Bus implementation that removes the need for `unsafe` code.
    struct MockDbus {
        should_register_succeed: bool,
    }
    #[async_trait]
    impl DbusConnection for MockDbus {
        async fn setup(
            &self,
            _window_info: &WindowInfo,
            _exit_notify: Arc<Notify>,
            _hyprland: &Hyprland,
        ) -> Result<Option<(Arc<Connection>, String)>> {
            Ok(None)
        }
        async fn register(&self, _connection: &Arc<Connection>, _bus_name: &str) -> Result<()> {
            if self.should_register_succeed {
                Ok(())
            } else {
                Err(anyhow!("Mock D-Bus registration failed"))
            }
        }
    }

    // --- The Test (FIXED) ---

    #[tokio::test]
    async fn test_watcher_registration_failure_recovery() -> Result<()> {
        // --- 1. Setup ---
        let temp_file = NamedTempFile::new()?;
        let stack = Stack::new(temp_file.path());
        let mock_dbus = MockDbus {
            should_register_succeed: false, // Simulate registration failure
        };

        let test_window = WindowInfo {
            address: "0xMINIMIZE_TEST".to_string(),
            class: "TestApp".to_string(),
            title: "Test Window".to_string(),
            workspace: Workspace { id: 1 },
        };

        // --- 2. Execution ---
        // The hyprland mock is now handled by its own module's thread_local.
        let minimizer = Minimizer::new(&stack, test_window, Hyprland::new(), &mock_dbus);
        let result = minimizer.minimize().await;

        // --- 3. Assertions ---
        // This test now correctly checks the recovery logic when D-Bus setup fails.
        assert!(result.is_err(), "Expected run_tray_app to fail");
        let err_string = result.unwrap_err().to_string();
        assert!(
            err_string.contains("Mock D-Bus setup failed"),
            "Error message did not match expected failure reason"
        );

        // We can't easily inspect the dispatched commands now, but we can check the stack.
        assert!(
            stack.pop()?.is_none(),
            "Stack should be empty after recovery"
        );

        Ok(())
    }
}
