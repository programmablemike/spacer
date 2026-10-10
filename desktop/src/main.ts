import "./style.css";
import {
  api,
  errorMessage,
  type Operation,
  type Project,
  type Snapshot,
  type Target,
} from "./api";

const app = document.querySelector<HTMLDivElement>("#app")!;
const esc = (value: string | number) =>
  String(value).replace(
    /[&<>"']/g,
    (char) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        char
      ]!,
  );
const folderName = (path: string) =>
  path.split(/[\\/]/).filter(Boolean).at(-1) || path;
const icon = (
  name: "folder" | "branch" | "arrow" | "refresh" | "plus" | "grid" | "copy",
) => {
  const paths = {
    folder: '<path d="M3 7V5h6l2 2h10v12H3z"/>',
    branch:
      '<circle cx="6" cy="4" r="2"/><circle cx="6" cy="20" r="2"/><circle cx="18" cy="6" r="2"/><path d="M6 6v12m0-6c7 0 12-1 12-4"/>',
    arrow: '<path d="M5 12h14m-6-6 6 6-6 6"/>',
    refresh:
      '<path d="M20 7v5h-5M4 17v-5h5M5 8a8 8 0 0 1 14-2l1 6M4 12l1 6a8 8 0 0 0 14-2"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    grid: '<rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/>',
    copy: '<rect x="8" y="8" width="12" height="13" rx="2"/><path d="M16 8V3H3v13h5"/>',
  };
  return `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name]}</svg>`;
};
interface Modal {
  kind: "space" | "project" | "register" | "change" | "confirm";
  title: string;
  description: string;
  submit: string;
  values: Record<string, string>;
  operation?: Operation;
  path?: string;
}
let snapshot: Snapshot | null = null;
let selectedSpace: string | null = null;
let selectedProject: string | null = null;
let workspacePath = "";
let filter = "";
let busy = false;
let error = "";
let notice = "";
let modal: Modal | null = null;
let recent: string[] = [];
try {
  const saved: unknown = JSON.parse(
    localStorage.getItem("spacer.recentRoots") || "[]",
  );
  if (Array.isArray(saved))
    recent = saved
      .filter((value): value is string => typeof value === "string")
      .slice(0, 5);
} catch {
  /* The workspace registry is independent of optional UI preferences. */
}

function remember(root: string) {
  recent = [root, ...recent.filter((path) => path !== root)].slice(0, 5);
  try {
    localStorage.setItem("spacer.recentRoots", JSON.stringify(recent));
  } catch {
    /* Optional preference. */
  }
}
function accept(next: Snapshot) {
  snapshot = next;
  if (!next.spaces.some((space) => space.name === selectedSpace))
    selectedSpace = null;
  if (
    !next.projects.some(
      (project) =>
        project.name === selectedProject && project.space === selectedSpace,
    )
  )
    selectedProject = null;
}
function project(): Project | undefined {
  return snapshot?.projects.find(
    (item) => item.space === selectedSpace && item.name === selectedProject,
  );
}
const attrs = (values: Record<string, string | undefined>) =>
  Object.entries(values)
    .filter(([, value]) => value !== undefined)
    .map(([key, value]) => `data-${key}="${esc(value!)}"`)
    .join(" ");
function button(
  label: string,
  action: string,
  style = "secondary",
  data: Record<string, string | undefined> = {},
  glyph = "",
) {
  return `<button type="button" class="button ${style}" data-action="${action}" ${attrs(data)} ${busy ? "disabled" : ""}>${glyph}${esc(label)}</button>`;
}
function status() {
  return `${error ? `<div class="message error" role="alert"><strong>Could not complete the request</strong><span>${esc(error)}</span>${button("Dismiss", "dismiss", "text")}</div>` : ""}
    ${notice ? `<div class="message notice" role="status">${esc(notice)}</div>` : ""}`;
}
function welcome() {
  return `<aside class="sidebar welcome-sidebar"><div class="brand"><img src="/logo-icon.svg" alt=""/>Spacer<span>ALPHA</span></div>
    <div class="welcome-story"><div class="eyebrow">ROOM TO BUILD</div><h2>Your projects.<br/>Their own space.</h2><p>Keep repositories organized and give every change a place to happen.</p>
    <ol><li><span>01</span>Choose a workspace</li><li><span>02</span>Bring your projects</li><li><span>03</span>Start a change</li></ol></div>
    <div class="sidebar-footer"><span class="status-dot"></span>Local files. Your Git repositories.</div></aside>
    <main class="welcome-main"><div class="welcome-content"><img class="hero-icon" src="/logo-icon.svg" alt="A yellow block docking into an organized grid"/>
    <div class="eyebrow">WELCOME TO SPACER</div><h1>A home for your<br/>next piece of work.</h1><p class="lede">Open a workspace to see your projects and worktrees, or create one in a folder of your choice.</p>
    ${status()}<form id="workspace-form" class="workspace-form"><label for="workspace-path">Workspace folder</label><div class="input-row"><input id="workspace-path" name="path" placeholder="Choose a folder…" value="${esc(workspacePath)}" required ${busy ? "disabled" : ""}/>${button("Browse…", "browse-workspace")}</div>
    <div class="form-actions"><button type="submit" class="button primary" ${busy ? "disabled" : ""}>${busy ? "Opening…" : "Open workspace"}${icon("arrow")}</button>${button("Create workspace", "initialize", "secondary")}</div><p class="hint">Creating a workspace adds a .spacer folder. Existing files stay in place.</p></form>
    ${recent.length ? `<section class="recent"><div class="eyebrow">RECENT WORKSPACES</div>${recent.map((root) => `<button class="recent-item" data-action="open-recent" data-root="${esc(root)}" ${busy ? "disabled" : ""}>${icon("folder")}<span><strong>${esc(folderName(root))}</strong><small>${esc(root)}</small></span>${icon("arrow")}</button>`).join("")}</section>` : ""}
    </div></main>`;
}
function sidebar() {
  const s = snapshot!;
  return `<aside class="sidebar"><div class="brand"><img src="/logo-icon.svg" alt=""/>Spacer<span>ALPHA</span></div>
    <div class="workspace-name"><span class="eyebrow">WORKSPACE</span><h2>${esc(folderName(s.root))}</h2><p title="${esc(s.root)}">${esc(s.root)}</p>${button("Switch workspace", "switch", "sidebar-button")}</div>
    <nav aria-label="Workspace"><button class="nav-item ${selectedSpace === null ? "selected" : ""}" data-action="overview" ${busy ? "disabled" : ""}>${icon("grid")}<span>Overview</span><span class="count">${s.projects.length}</span></button>
    <div class="nav-label"><span>SPACES</span><button class="icon-button" data-action="new-space" aria-label="Create space" ${busy ? "disabled" : ""}>${icon("plus")}</button></div>
    ${s.spaces.map((space) => `<button class="nav-item ${selectedSpace === space.name ? "selected" : ""}" data-action="select-space" data-space="${esc(space.name)}" ${busy ? "disabled" : ""}><span class="space-mark">${esc(space.name.charAt(0).toUpperCase())}</span><span>${esc(space.name)}</span><span class="count">${s.projects.filter((p) => p.space === space.name).length}</span></button>`).join("")}
    ${!s.spaces.length ? '<p class="sidebar-hint">Create a space to group your projects.</p>' : ""}</nav>
    <div class="sidebar-footer"><span class="status-dot"></span>Local workspace<span class="footer-note">Git stays the source of truth.</span></div></aside>`;
}
function projectCards() {
  const s = snapshot!;
  const projects = s.projects.filter(
    (p) =>
      (!selectedSpace || p.space === selectedSpace) &&
      `${p.name} ${p.space}`.toLowerCase().includes(filter.toLowerCase()),
  );
  if (!projects.length)
    return `<div class="empty"><div class="empty-icon">${icon("folder")}</div><h3>${filter ? "No matching projects" : "Give your work a place"}</h3><p>${filter ? "Try another project or space name." : "Create a new repository, or register one you already have."}</p>${!filter && s.spaces.length ? button("Register existing project", "register", "secondary") : ""}</div>`;
  return `<div class="project-grid">${projects
    .map((p) => {
      const changes = p.worktrees.filter((tree) => !tree.isPrimary).length;
      const primary = p.worktrees.find((tree) => tree.isPrimary);
      const data = { space: p.space, project: p.name };
      return `<article class="project-card"><button class="project-select" data-action="select-project" ${attrs(data)} ${busy ? "disabled" : ""}><div class="card-top"><span class="folder-icon">${icon("folder")}</span><span class="badge">${esc(p.space)}</span></div><h3>${esc(p.name)}</h3><p class="path" title="${esc(p.path)}">${esc(p.path)}</p></button>
    <div class="card-footer"><span class="branch-label">${icon("branch")}${esc(p.error ? "Read error" : primary?.branch || "No commits yet")}</span><button class="link" data-action="select-project" ${attrs(data)} ${busy ? "disabled" : ""}>${changes} ${changes === 1 ? "change" : "changes"} ${icon("arrow")}</button></div>
    ${p.error ? `<p class="card-error" role="status">${esc(p.error.message)}</p>` : ""}</article>`;
    })
    .join("")}</div>`;
}
function overview() {
  const s = snapshot!;
  const projects = s.projects.filter(
    (p) => !selectedSpace || p.space === selectedSpace,
  );
  const changes = projects.reduce(
    (sum, p) => sum + p.worktrees.filter((tree) => !tree.isPrimary).length,
    0,
  );
  return `<div class="page-heading"><div><div class="eyebrow">${selectedSpace ? "SPACE" : "YOUR WORK, ORGANIZED"}</div><h1>${esc(selectedSpace || "Workspace overview")}</h1><p class="lede">${selectedSpace ? "Repositories together. Changes with room to grow." : "One place to find your projects and pick up where you left off."}</p></div>
    <div class="heading-actions">${!s.spaces.length ? button("Create space", "new-space", "primary", {}, icon("plus")) : `${button("Register project", "register")}${button("New project", "new-project", "primary", {}, icon("plus"))}`}</div></div>
    ${selectedSpace ? `<div class="toolbar">${button("Open folder", "open-space", "text", {}, icon("folder"))}${button("Copy path", "copy-space", "text", {}, icon("copy"))}${button(s.activeSpace === selectedSpace ? "Default space" : "Use as default", "use-space", "text")}${button("Unregister space", "unregister-space", "text danger")}</div>` : ""}
    <div class="stats"><div><strong>${selectedSpace ? projects.length : s.spaces.length}</strong><span>${selectedSpace ? "Projects in this space" : "Spaces"}</span></div><div><strong>${selectedSpace ? changes : s.projects.length}</strong><span>${selectedSpace ? "Linked worktrees" : "Projects"}</span></div><div><strong>${selectedSpace ? projects.filter((p) => !!p.error).length : changes}</strong><span>${selectedSpace ? "Projects needing attention" : "Linked worktrees"}</span></div></div>
    <div class="section-heading"><h2>Projects <span class="muted">${projects.length}</span></h2><label class="search"><span class="sr-only">Filter projects</span><input id="filter" placeholder="Find a project…" value="${esc(filter)}"/></label></div><div id="project-list">${projectCards()}</div>`;
}
function projectView(p: Project) {
  const s = snapshot!;
  const changes = p.worktrees.filter((tree) => !tree.isPrimary);
  const primary = p.worktrees.find((tree) => tree.isPrimary);
  const tracked = s.trackedChanges.filter(
    (c) => c.space === p.space && c.project === p.name,
  );
  return `<div class="page-heading"><div><div class="eyebrow">PROJECT / ${esc(p.space)}</div><h1>${esc(p.name)}</h1><p class="path full-path">${esc(p.path)}</p></div><div class="heading-actions">${button("Start change", "new-change", "primary", {}, icon("plus"))}</div></div>
    <div class="toolbar">${button("Open folder", "open-project", "text", {}, icon("folder"))}${button("Copy path", "copy-project", "text", {}, icon("copy"))}${button(s.activeSpace === p.space && s.activeProject === p.name ? "Default project" : "Use as default", "use-project", "text")}${button("Unregister project", "unregister-project", "text danger")}</div>
    ${p.error ? `<div class="message error" role="alert"><strong>Repository needs attention</strong><span>${esc(p.error.message)}</span><span>Its registration is preserved. Repair the repository, refresh, or unregister it.</span></div>` : ""}
    <section class="primary-checkout"><span class="folder-icon">${icon("folder")}</span><div><h2>Primary checkout</h2><p>${primary?.branch ? esc(primary.branch) : "Create an initial commit with Git before starting a change."}</p></div><span class="badge">${primary?.branch ? "PRIMARY" : "GETTING STARTED"}</span></section>
    <div class="section-heading"><h2>Linked worktrees <span class="muted">${changes.length}</span></h2></div>
    <p class="section-description">An isolated checkout for each change. Finishing removes a clean checkout and keeps its branch.</p>
    ${changes.length ? `<div class="worktrees">${changes.map((tree) => `<article class="worktree"><div class="worktree-icon">${icon("branch")}</div><div class="worktree-info"><h3>${esc(tree.branch || "Detached checkout")}${!tree.branch ? '<span class="badge">DETACHED</span>' : ""}</h3><p class="path full-path">${esc(tree.path)}</p></div><div class="row-actions">${button("Open", "open-tree", "secondary", { path: tree.path })}${button("Copy path", "copy-tree", "text", { path: tree.path }, icon("copy"))}${tree.branch ? button("Finish", "finish", "text", { branch: tree.branch, path: tree.path }) : '<span class="hint">Manage with Git</span>'}</div></article>`).join("")}</div>` : `<div class="empty compact"><div class="empty-icon">${icon("branch")}</div><h3>No changes in flight</h3><p>Start a change to create a branch and its own working directory.</p></div>`}
    ${tracked.length ? `<section class="legacy"><h2>Legacy tracking</h2><p>These are saved records, not Git worktrees. Removing one only removes its tracking entry.</p>${tracked.map((change) => `<div class="legacy-row"><strong>${esc(change.name)}</strong>${button("Remove tracking", "remove-tracked", "text", { branch: change.name })}</div>`).join("")}</section>` : ""}`;
}
function field(
  label: string,
  name: string,
  placeholder: string,
  required = true,
) {
  return `<label for="field-${name}">${esc(label)}</label><input id="field-${name}" name="${name}" value="${esc(modal?.values[name] || "")}" placeholder="${esc(placeholder)}" ${required ? "required" : ""} ${busy ? "disabled" : ""}/>`;
}
function dialog() {
  if (!modal) return "";
  const m = modal;
  const spaceField =
    m.kind === "project" || m.kind === "register"
      ? `<label for="field-space">Space</label><select id="field-space" name="space" required ${busy ? "disabled" : ""}>${snapshot!.spaces.map((space) => `<option ${space.name === (m.values.space || selectedSpace) ? "selected" : ""} value="${esc(space.name)}">${esc(space.name)}</option>`).join("")}</select>`
      : "";
  const pathField = ["space", "project", "register"].includes(m.kind)
    ? `<label for="field-path">${m.kind === "register" ? "Repository folder" : "Custom folder (optional)"}</label><div class="input-row"><input id="field-path" name="path" value="${esc(m.values.path || "")}" placeholder="${m.kind === "register" ? "Choose the primary repository folder…" : "Use the default workspace layout"}" ${m.kind === "register" ? "required" : ""} ${busy ? "disabled" : ""}/>${button("Browse…", "browse-path")}</div>`
    : "";
  return `<dialog id="dialog" aria-labelledby="dialog-title"><form id="operation-form"><div class="dialog-heading"><span class="eyebrow">${m.kind === "confirm" ? "REVIEW ACTION" : "MAKE ROOM"}</span><h2 id="dialog-title">${esc(m.title)}</h2><p>${esc(m.description)}</p>${m.path ? `<code class="full-path">${esc(m.path)}</code>` : ""}</div>${error ? `<div class="message error" role="alert">${esc(error)}</div>` : ""}
    ${m.kind !== "confirm" ? `${spaceField}${field(m.kind === "change" ? "Branch name" : "Name", "name", m.kind === "change" ? "feat/your-next-idea" : "A short, descriptive name")}${pathField}` : ""}
    <div class="dialog-actions">${button("Cancel", "cancel")}<button type="submit" class="button primary" ${busy ? "disabled" : ""}>${busy ? "Working…" : esc(m.submit)}</button></div></form></dialog>`;
}
function render() {
  app.setAttribute("aria-busy", String(busy));
  app.innerHTML = !snapshot
    ? welcome()
    : `${sidebar()}<div class="workspace-content"><header class="topbar"><div class="breadcrumbs"><button data-action="overview" ${busy ? "disabled" : ""}>${esc(folderName(snapshot.root))}</button>${selectedSpace ? `<span>/</span><button data-action="select-space" data-space="${esc(selectedSpace)}" ${busy ? "disabled" : ""}>${esc(selectedSpace)}</button>` : ""}${selectedProject ? `<span>/</span><strong>${esc(selectedProject)}</strong>` : ""}</div>${button(busy ? "Working…" : "Refresh", "refresh", "secondary", {}, icon("refresh"))}</header><main class="workspace-main">${modal ? "" : status()}${project() ? projectView(project()!) : overview()}</main><footer class="workspace-footer">${busy ? "Working locally…" : "Refresh to pick up changes made outside Spacer."}${button("Open workspace folder", "open-root", "text")}</footer></div>${dialog()}`;
  if (modal) {
    const element = document.querySelector<HTMLDialogElement>("#dialog")!;
    element.showModal();
    element.addEventListener("cancel", (event) => {
      event.preventDefault();
      if (!busy) {
        modal = null;
        error = "";
        render();
      }
    });
    element.querySelector<HTMLInputElement>("input")?.focus();
  }
}
async function run(work: () => Promise<void>) {
  if (busy) return;
  busy = true;
  error = "";
  notice = "";
  render();
  try {
    await work();
  } catch (failure) {
    error = errorMessage(failure);
  } finally {
    busy = false;
    render();
  }
}
async function openWorkspace(root: string, initialize: boolean) {
  workspacePath = root;
  await run(async () => {
    const next = await api.open(root, initialize);
    selectedSpace = null;
    selectedProject = null;
    filter = "";
    accept(next);
    remember(next.root);
    notice = initialize
      ? "Workspace ready. Create a space to get started."
      : "";
  });
}
function showForm(kind: Modal["kind"]) {
  if (kind !== "space" && !snapshot!.spaces.length) kind = "space";
  const config = {
    space: [
      "Create a space",
      "A space groups related projects, such as an organization or a personal collection.",
      "Create space",
    ],
    project: [
      "Create a project",
      "Initialize a Git repository in a new or empty folder. Make your first commit with Git before starting a change.",
      "Create project",
    ],
    register: [
      "Register a project",
      "Add an existing primary Git repository. Its files and history stay where they are.",
      "Register project",
    ],
    change: [
      "Start a change",
      "Create a new branch from HEAD and a linked worktree. Existing branches are never reset or reused.",
      "Start change",
    ],
    confirm: ["", "", ""],
  }[kind];
  modal = {
    kind,
    title: config[0],
    description: config[1],
    submit: config[2],
    values: { space: selectedSpace || snapshot!.spaces[0]?.name || "" },
  };
  error = "";
  notice = "";
  render();
}
function confirm(
  operation: Operation,
  title: string,
  description: string,
  submit: string,
  path?: string,
) {
  modal = {
    kind: "confirm",
    title,
    description,
    submit,
    operation,
    path,
    values: {},
  };
  error = "";
  notice = "";
  render();
}
async function mutation(operation: Operation) {
  const root = snapshot!.root;
  await run(async () => {
    accept(await api.mutate(root, operation));
    modal = null;
    notice =
      operation.kind === "finishChange"
        ? "Change finished. Its branch and commits are retained."
        : operation.kind.startsWith("unregister")
          ? "Registration removed. All files are preserved."
          : "Workspace updated.";
  });
}
function targetFor(action: string, data: DOMStringMap): Target {
  if (action.endsWith("root")) return { kind: "workspace" };
  if (action.endsWith("space")) return { kind: "space", space: selectedSpace! };
  const p = project()!;
  if (action.endsWith("tree"))
    return {
      kind: "worktree",
      space: p.space,
      project: p.name,
      path: data.path!,
    };
  return { kind: "project", space: p.space, project: p.name };
}
app.addEventListener("input", (event) => {
  const input = event.target as HTMLInputElement;
  if (input.id === "workspace-path") workspacePath = input.value;
  if (input.id === "filter") {
    filter = input.value;
    document.querySelector("#project-list")!.innerHTML = projectCards();
  }
  if (modal && input.name) modal.values[input.name] = input.value;
});
app.addEventListener("change", (event) => {
  const input = event.target as HTMLSelectElement;
  if (modal && input.name) modal.values[input.name] = input.value;
});
app.addEventListener("click", async (event) => {
  const element = (event.target as Element).closest<HTMLElement>(
    "[data-action]",
  );
  if (!element || busy) return;
  const action = element.dataset.action!;
  const data = element.dataset;
  const p = project();
  if (action === "dismiss") {
    error = "";
    render();
  } else if (action === "cancel") {
    modal = null;
    error = "";
    render();
  } else if (action === "switch") {
    snapshot = null;
    selectedSpace = null;
    selectedProject = null;
    error = "";
    notice = "";
    render();
  } else if (
    action === "overview" ||
    action === "select-space" ||
    action === "select-project"
  ) {
    selectedSpace = action === "overview" ? null : data.space!;
    selectedProject = action === "select-project" ? data.project! : null;
    filter = "";
    error = "";
    notice = "";
    render();
  } else if (action === "browse-workspace" || action === "browse-path") {
    await run(async () => {
      const path = await api.pickFolder();
      if (typeof path === "string") {
        if (action === "browse-workspace") workspacePath = path;
        else if (modal) modal.values.path = path;
      }
    });
  } else if (action === "open-recent") await openWorkspace(data.root!, false);
  else if (action === "initialize") {
    if (workspacePath.trim()) await openWorkspace(workspacePath.trim(), true);
    else
      document
        .querySelector<HTMLInputElement>("#workspace-path")!
        .reportValidity();
  } else if (action === "refresh")
    await run(async () => {
      accept(await api.read(snapshot!.root));
      notice = "Workspace refreshed.";
    });
  else if (action === "new-space") showForm("space");
  else if (action === "new-project") showForm("project");
  else if (action === "register") showForm("register");
  else if (action === "new-change") showForm("change");
  else if (action === "use-space")
    await mutation({ kind: "useSpace", space: selectedSpace! });
  else if (action === "use-project")
    await mutation({ kind: "useProject", project: p!.name, space: p!.space });
  else if (action === "unregister-space")
    confirm(
      { kind: "unregisterSpace", space: selectedSpace! },
      `Unregister ${selectedSpace}?`,
      "The directory stays in place. All projects must be unregistered first.",
      "Unregister space",
    );
  else if (action === "unregister-project")
    confirm(
      { kind: "unregisterProject", project: p!.name, space: p!.space },
      `Unregister ${p!.name}?`,
      "Remove this project from Spacer. Its files, branches, and worktrees are preserved and reappear if you register it again.",
      "Unregister project",
      p!.path,
    );
  else if (action === "finish")
    confirm(
      {
        kind: "finishChange",
        name: data.branch!,
        project: p!.name,
        space: p!.space,
      },
      `Finish ${data.branch}?`,
      "Remove this linked checkout only if it is clean. The branch and all commits are retained. Modified, untracked, ignored, and locked worktrees are refused.",
      "Finish change",
      data.path,
    );
  else if (action === "remove-tracked")
    confirm(
      {
        kind: "removeTrackedChange",
        name: data.branch!,
        project: p!.name,
        space: p!.space,
      },
      `Remove tracking for ${data.branch}?`,
      "Only the legacy tracking entry is removed. No Git operation is performed and no files are deleted.",
      "Remove tracking",
    );
  else if (action.startsWith("open-") || action.startsWith("copy-")) {
    const target = targetFor(action, data);
    await run(async () => {
      if (action.startsWith("open-")) await api.reveal(snapshot!.root, target);
      else {
        await api.copy(snapshot!.root, target);
        notice = "Path copied.";
      }
    });
  }
});
app.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (busy) return;
  const form = event.target as HTMLFormElement;
  if (form.id === "workspace-form") {
    await openWorkspace(workspacePath.trim(), false);
    return;
  }
  if (!modal || form.id !== "operation-form") return;
  const values = Object.fromEntries(new FormData(form)) as Record<
    string,
    string
  >;
  modal.values = values;
  const p = project();
  let operation = modal.operation;
  if (modal.kind === "space")
    operation = {
      kind: "createSpace",
      name: values.name.trim(),
      path: values.path.trim() || null,
    };
  else if (modal.kind === "project")
    operation = {
      kind: "createProject",
      name: values.name.trim(),
      space: values.space,
      path: values.path.trim() || null,
    };
  else if (modal.kind === "register")
    operation = {
      kind: "registerProject",
      name: values.name.trim(),
      space: values.space,
      path: values.path.trim(),
    };
  else if (modal.kind === "change")
    operation = {
      kind: "startChange",
      name: values.name.trim(),
      project: p!.name,
      space: p!.space,
    };
  if (operation) await mutation(operation);
});
render();
