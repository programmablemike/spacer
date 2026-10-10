import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

export interface BridgeError {
  kind: string;
  message: string;
}
export interface Space {
  name: string;
  path: string;
}
export interface Checkout {
  path: string;
  branch: string | null;
  isPrimary: boolean;
}
export interface Project {
  name: string;
  space: string;
  path: string;
  worktrees: Checkout[];
  error: BridgeError | null;
}
export interface TrackedChange {
  name: string;
  project: string;
  space: string;
}
export interface Snapshot {
  root: string;
  spaces: Space[];
  projects: Project[];
  trackedChanges: TrackedChange[];
  activeSpace: string | null;
  activeProject: string | null;
}
export type Operation =
  | { kind: "createSpace"; name: string; path: string | null }
  | { kind: "createProject"; name: string; space: string; path: string | null }
  | { kind: "registerProject"; name: string; space: string; path: string }
  | { kind: "unregisterSpace" | "useSpace"; space: string }
  | { kind: "unregisterProject" | "useProject"; project: string; space: string }
  | {
      kind: "startChange" | "finishChange" | "removeTrackedChange";
      name: string;
      project: string;
      space: string;
    };
export type Target =
  | { kind: "workspace" }
  | { kind: "space"; space: string }
  | { kind: "project"; space: string; project: string }
  | { kind: "worktree"; space: string; project: string; path: string };

export const api = {
  open: (root: string, initialize: boolean) =>
    invoke<Snapshot>("open_workspace", { root, initialize }),
  read: (root: string) => invoke<Snapshot>("read_workspace", { root }),
  mutate: (root: string, operation: Operation) =>
    invoke<Snapshot>("mutate_workspace", { root, operation }),
  reveal: (root: string, target: Target) =>
    invoke<void>("reveal_target", { root, target }),
  copy: async (root: string, target: Target) => {
    const path = await invoke<string>("resolve_target", { root, target });
    await writeText(path);
  },
  pickFolder: () =>
    open({ directory: true, multiple: false, title: "Choose a folder" }),
};
export function errorMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "message" in error)
    return String(error.message);
  if (typeof error === "string") return error;
  return "The operation could not complete. Refresh the workspace and try again.";
}
