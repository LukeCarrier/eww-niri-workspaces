use niri_ipc::{socket::Socket, Event, Request, Response, Window, Workspace};

mod serializable;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = State::new();
    let niri_socket_env = std::env::var("NIRI_SOCKET");
    let mut connection = if let Ok(niri_socket) = niri_socket_env {
        Socket::connect_to(niri_socket).unwrap()
    } else {
        Socket::connect().unwrap()
    };
    let reply = connection.send(Request::EventStream)?;
    if matches!(reply, Ok(Response::Handled)) {
        let mut read_one_event = connection.read_events();
        while let Ok(event) = read_one_event() {
            state.update_with_event(event);
            let serializable_state = serializable::SerializableState::from(&state);
            let json = serde_json::to_string(&serializable_state).unwrap();
            println!("{}", json);
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
struct State {
    workspaces: Vec<Workspace>,
    windows: Vec<Window>,
}

impl State {
    fn new() -> Self {
        Self::default()
    }

    /// https://yalter.github.io/niri/niri_ipc/enum.Event.html
    fn update_with_event(&mut self, e: Event) {
        match e {
            Event::WorkspacesChanged { workspaces } => self.workspaces = workspaces,
            Event::WorkspaceActivated { id, focused } => {
                // If this workspace is focused, unfocus all others
                if focused {
                    for workspace in &mut self.workspaces {
                        workspace.is_focused = false;
                    }
                }

                // Find and update the activated workspace
                let activated_output = match self.workspaces.iter_mut().find(|w| w.id == id) {
                    Some(workspace) => {
                        workspace.is_active = true;
                        workspace.is_focused = focused;
                        workspace.output.clone()
                    }
                    None => panic!("Workspace not found"),
                };

                // Deactivate other workspaces on the same output
                if activated_output.is_some() {
                    for workspace in &mut self.workspaces {
                        if workspace.id != id && workspace.output == activated_output {
                            workspace.is_active = false;
                        }
                    }
                }
            }
            Event::WorkspaceActiveWindowChanged {
                workspace_id,
                active_window_id,
            } => {
                if let Some(workspace) = self.workspaces.iter_mut().find(|w| w.id == workspace_id) {
                    workspace.active_window_id = active_window_id;
                }
            }
            Event::WindowsChanged { windows } => self.windows = windows,
            Event::WindowOpenedOrChanged { window } => {
                if window.is_focused {
                    // All other windows become not focused
                    for window in self.windows.iter_mut() {
                        window.is_focused = false;
                    }
                }

                // Change or add window
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == window.id) {
                    *w = window;
                } else {
                    self.windows.push(window);
                }
            }
            Event::WindowClosed { id } => {
                self.windows.retain(|w| w.id != id);
            }
            Event::WindowFocusChanged { id } => {
                // All other windows become not focused
                for window in self.windows.iter_mut() {
                    window.is_focused = false;
                }

                // If a window is meant to be focused
                if let Some(id) = id {
                    if let Some(window) = self.windows.iter_mut().find(|w| w.id == id) {
                        window.is_focused = true;
                    }
                }
            }
            Event::KeyboardLayoutsChanged { .. } => { /* Do nothing */ }
            Event::KeyboardLayoutSwitched { .. } => { /* Do nothing */ }
            e => eprintln!("Unhandled event: {:?}", e),
        }
    }
}
