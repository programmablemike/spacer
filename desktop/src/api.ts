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

export interface CommitIdentity {
  id: string;
  label: string;
  name: string;
  email: string;
}
export interface Connector {
  id: string;
  label: string;
  login: string;
  accountId: string;
  source: string;
}
export interface Catalog {
  identities: CommitIdentity[];
  connectors: Connector[];
}
export interface RemoteRepository {
  id: string;
  fullName: string;
  private: boolean;
}
export interface LoginChallenge {
  id: string;
  userCode: string;
  verificationUri: string;
  interval: number;
}
export interface SetupPlan {
  id: string;
  repository: string;
  project: string;
  path: string;
  connector: Connector;
  identity: CommitIdentity | null;
}
export type ConnectionRequest =
  | { kind: "catalog" | "existing" }
  | { kind: "addIdentity"; label: string; name: string; email: string }
  | { kind: "removeIdentity"; identity: string }
  | { kind: "link"; label: string; login: string }
  | { kind: "beginLogin"; label: string }
  | { kind: "pollLogin" | "cancelLogin"; id: string }
  | { kind: "disconnect" | "repositories"; connector: string };
export type ConnectionReply =
  | { kind: "catalog"; value: Catalog }
  | { kind: "existing"; value: string[] }
  | { kind: "login"; value: LoginChallenge }
  | { kind: "connected"; value: Connector | null }
  | { kind: "repositories"; value: RemoteRepository[] }
  | { kind: "done" };
export const setupApi = {
  request: (request: ConnectionRequest) =>
    invoke<ConnectionReply>("connection_request", { request }),
  authorize: () => invoke<void>("open_github_authorization"),
  plan: (
    root: string,
    space: string,
    connector: string,
    identity: string | null,
    repositories: { id: string; project: string }[],
  ) =>
    invoke<SetupPlan[]>("plan_setup", {
      root,
      space,
      connector,
      identity,
      repositories,
    }),
  execute: (root: string, id: string) =>
    invoke<void>("execute_setup", { root, id }),
  identity: (root: string, space: string, project: string) =>
    invoke<[string | null, string | null]>("read_commit_identity", {
      root,
      space,
      project,
    }),
  applyIdentity: (
    root: string,
    space: string,
    project: string,
    identity: string,
  ) =>
    invoke<void>("apply_commit_identity", { root, space, project, identity }),
};
