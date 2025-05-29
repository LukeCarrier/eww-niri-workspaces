use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

use crate::State;

/// Serializable state ready to be consumed by Eww as Json
#[derive(Serialize)]
pub(crate) struct SerializableState {
    outputs: BTreeMap<String, Output>,
}

#[derive(Serialize)]
struct Output {
    #[serde(serialize_with = "ordered_map_as_list")]
    workspaces: BTreeMap<u64, Workspace>,
}
#[derive(Serialize)]
struct Workspace {
    id: u64,
    index: u8,
    name: Option<String>,
    #[serde(serialize_with = "ordered_map_as_list")]
    windows: BTreeMap<u64, Window>,
    is_active: bool,
}

#[derive(Serialize)]
struct Window {
    id: u64,
    is_focused: bool,
    title: Option<String>,
}

fn ordered_map_as_list<S, T>(
    map: &BTreeMap<T, impl Serialize>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let list: Vec<_> = map.values().collect();
    list.serialize(serializer)
}

impl From<&State> for SerializableState {
    fn from(state: &State) -> Self {
        // first create the workspaces - without windows, then populate the windows
        let mut outputs = BTreeMap::<String, Output>::new();
        for workspace in state.workspaces.iter() {
            let output_name = if let Some(output) = &workspace.output {
                output
            } else {
                continue;
            };

            let output = outputs
                .entry(output_name.clone())
                .or_insert_with(|| Output {
                    workspaces: BTreeMap::new(),
                });

            output.workspaces.insert(
                workspace.id,
                Workspace {
                    id: workspace.id,
                    index: workspace.idx,
                    name: workspace.name.clone(),
                    windows: BTreeMap::new(),
                    is_active: workspace.is_active,
                },
            );
        }

        // populate the windows
        for window in state.windows.iter() {
            // We only care about non-floating windows
            if window.is_floating {
                continue;
            }
            // We only care about windows with a workspace (that exists)
            let workspace = match window.workspace_id {
                Some(workspace_id) => outputs
                    .values_mut()
                    .flat_map(|output| output.workspaces.values_mut())
                    .find(|workspace| workspace.id == workspace_id)
                    .expect("Workspace id set for window not found in state's workspaces"),
                None => continue,
            };

            workspace.windows.insert(
                window.id,
                Window {
                    id: window.id,
                    title: window.title.clone(),
                    is_focused: window.is_focused,
                },
            );
        }

        SerializableState { outputs }
    }
}
