import {
  api,
  errorMessage,
  setupApi,
  type Catalog,
  type LoginChallenge,
  type RemoteRepository,
  type SetupPlan,
  type Space,
  type Project,
} from "./api";

const esc = (s: string) =>
  s.replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ]!,
  );
export async function mountSetup(
  app: HTMLElement,
  root: string,
  spaces: Space[],
  close: () => void,
  project?: Project,
) {
  const element = document.createElement("main");
  element.className = "setup-screen";
  app.replaceChildren(element);
  let catalog: Catalog = { identities: [], connectors: [] };
  let page:
    "connections" | "repositories" | "identity" | "review" | "progress" =
    project ? "identity" : "connections";
  let connector = "";
  let identity = "";
  let space = spaces[0]?.name || "";
  let repositories: RemoteRepository[] = [];
  let existing: string[] = [];
  const selected = new Set<string>();
  const projectNames = new Map<string, string>();
  let plans: SetupPlan[] = [];
  const results = new Map<string, string>();
  let challenge: LoginChallenge | null = null;
  let generation = 0;
  let busy = false;
  let error = "";
  let filter = "";
  let label = "Personal GitHub";
  let identityLabel = "";
  let commitName = "";
  let commitEmail = "";
  let current: [string | null, string | null] = [null, null];
  let cancelled = false;
  let newSpace = "";
  let existingLogin = "";
  const button = (text: string, action: string, extra = "", enabled = true) =>
    `<button type="button" class="button ${extra}" data-setup="${action}" ${busy || !enabled ? "disabled" : ""}>${text}</button>`;
  async function refreshCatalog() {
    const reply = await setupApi.request({ kind: "catalog" });
    if (reply.kind !== "catalog")
      throw new Error("Unexpected account response");
    catalog = reply.value;
  }
  function render() {
    const heading = {
      connections: "Connect GitHub",
      repositories: "Choose repositories",
      identity: "Choose commit authorship",
      review: "Review project setup",
      progress: "Setting up projects",
    }[page];
    element.innerHTML = `<header class="page-heading"><div><div class="eyebrow">${project ? "PROJECT IDENTITY" : "SET UP YOUR WORKSPACE"}</div><h1>${heading}</h1><p class="path full-path">${esc(root)}</p></div>${button(page === "progress" ? "Open workspace" : "Use local repositories", "close")}</header>
      ${error ? `<div class="message error" role="alert">${esc(error)}</div>` : ""}
      <nav class="setup-steps" aria-label="Setup progress">${["connections", "repositories", "identity", "review"].map((s, i) => `<span ${s === page ? 'aria-current="step"' : ""}>${i + 1}. ${["Connector", "Repositories", "Identity", "Review"][i]}</span>`).join("")}</nav>
      ${
        page === "connections"
          ? `<p>Choose the account used to discover and clone repositories. Commit authorship is selected separately.</p>
        <div class="setup-cards">${catalog.connectors.map((c) => `<article class="primary-checkout"><div><h2>${esc(c.label)}</h2><p>GitHub · ${esc(c.login)} · ${c.source === "keychain" ? "System credential manager" : "GitHub CLI"}</p></div><div>${button("Use connector", `connector:${c.id}`, "primary")}${button("Disconnect", `disconnect:${c.id}`, "text danger")}</div></article>`).join("")}</div>
        <section class="setup-form"><h2>Add a connector</h2><p>GitHub OAuth sign-in requests repository access, including private repositories. The repo scope permits read and write access within your GitHub permissions. No repository installation is needed; organizations may require approval.</p><label>Connection label<input data-field="label" value="${esc(label)}" maxlength="256"/></label>
        ${challenge ? `<p>Enter <strong class="device-code">${esc(challenge.userCode)}</strong> at <a href="https://github.com/login/device" data-setup="browser">github.com/login/device</a>. Choose the account you intend to connect.</p><p role="status">Waiting for authorization…</p>${button("Open browser", "browser")}${button("Cancel sign-in", "cancel-login")}` : `${button("Sign in with GitHub", "login", "primary")}${button("Find existing GitHub CLI accounts", "existing")}`}
        ${existing.length ? `<label>Existing account<select aria-label="Existing account" data-field="existing">${existing.map((a) => `<option value="${esc(a)}" ${a === existingLogin ? "selected" : ""}>${esc(a)}</option>`).join("")}</select></label>${button("Use existing credentials", "link")}` : ""}</section>`
          : ""
      }
      ${
        page === "repositories"
          ? `<section class="setup-form"><label>Space<select aria-label="Space" data-field="space">${spaces.map((s) => `<option value="${esc(s.name)}" ${s.name === space ? "selected" : ""}>${esc(s.name)}</option>`).join("")}</select></label><div class="input-row"><input data-field="new-space" value="${esc(newSpace)}" placeholder="New space name"/>${button("Create space", "create-space")}</div><label>Find a repository<input data-field="filter" value="${esc(filter)}" placeholder="Owner or repository name"/></label><div class="repository-selection">${
              repositories
                .filter((r) =>
                  r.fullName.toLowerCase().includes(filter.toLowerCase()),
                )
                .map(
                  (r) =>
                    `<div><label class="repository-choice"><input type="checkbox" data-repository="${r.id}" ${selected.has(r.id) ? "checked" : ""}/><span>${esc(r.fullName)}</span><span class="badge">${r.private ? "PRIVATE" : "PUBLIC"}</span></label>${selected.has(r.id) ? `<label>Local project name<input aria-label="Local name for ${esc(r.fullName)}" data-project-name="${r.id}" value="${esc(projectNames.get(r.id) ?? r.fullName.split("/").at(-1)!)}"/></label>` : ""}</div>`,
                )
                .join("") ||
              "<p>No repositories are visible. Check repository permissions, OAuth App organization approval, and SSO.</p>"
            }</div>${button("Back", "back")}${button("Choose identity", "identity", "primary", selected.size > 0 && !!space)}</section>`
          : ""
      }
      ${
        page === "identity"
          ? `<section class="setup-form"><p>${project ? `Current authorship: <strong>${esc(current[0] || "unset")} &lt;${esc(current[1] || "unset")}&gt;</strong>. Apply changes only after reviewing the exact values below.` : "Choose the name and email that Git records in commits. Connecting GitHub does not set these values."}</p><label>Commit identity<select aria-label="Commit identity" data-field="identity"><option value="">Keep existing Git configuration</option>${catalog.identities.map((i) => `<option value="${i.id}" ${identity === i.id ? "selected" : ""}>${esc(i.label)} — ${esc(i.name)} &lt;${esc(i.email)}&gt;</option>`).join("")}</select></label>
      <h2>Create an identity</h2><label>Identity label<input data-field="identityLabel" value="${esc(identityLabel)}"/></label><label>Commit name<input data-field="commitName" value="${esc(commitName)}"/></label><label>Commit email<input type="email" data-field="commitEmail" value="${esc(commitEmail)}"/></label>${button("Save identity", "add-identity")}${current[0] && current[1] ? button("Use current Git values", "use-current") : ""}
      ${project ? button("Apply selected identity to this repository", "apply-identity", "primary", !!identity) : `${button("Back", "back")}${button("Review setup", "review", "primary")}`}</section>`
          : ""
      }
      ${page === "review" ? `<p>These repositories will be cloned and registered in <strong>${esc(space)}</strong>. A selected identity changes only repository-local authorship.</p><div class="table-scroll"><table><thead><tr><th>Repository</th><th>Destination</th><th>Connector</th><th>Commit authorship</th></tr></thead><tbody>${plans.map((p) => `<tr><td>${esc(p.repository)}</td><td class="full-path">${esc(p.path)}</td><td>${esc(p.connector.label)} (${esc(p.connector.login)})</td><td>${p.identity ? `${esc(p.identity.name)} &lt;${esc(p.identity.email)}&gt;` : "Existing Git configuration"}</td></tr>`).join("")}</tbody></table></div>${button("Back", "back")}${button(`Set up ${plans.length} project${plans.length === 1 ? "" : "s"}`, "execute", "primary")}` : ""}
      ${page === "progress" ? `<p>Completed projects remain available. If a checkout needs recovery, its files are preserved.</p><div role="status">${plans.map((p) => `<article class="primary-checkout"><div><h2>${esc(p.repository)}</h2><p class="full-path">${esc(p.path)}</p><p>${esc(results.get(p.id) || "Waiting")}</p></div></article>`).join("")}</div><button type="button" class="button" data-setup="stop" ${cancelled ? "disabled" : ""}>${cancelled ? "Stopping after current project…" : "Stop before next project"}</button>${button("Review remaining projects", "retry")}` : ""}`;
  }
  async function work(fn: () => Promise<void>) {
    if (busy) return;
    busy = true;
    error = "";
    render();
    try {
      await fn();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      busy = false;
      render();
    }
  }
  async function poll(ticket: LoginChallenge, nonce: number) {
    await new Promise((resolve) => setTimeout(resolve, ticket.interval * 1000));
    if (nonce !== generation || !challenge || !element.isConnected) return;
    try {
      const reply = await setupApi.request({
        kind: "pollLogin",
        id: ticket.id,
      });
      if (nonce !== generation) return;
      if (reply.kind === "connected" && reply.value) {
        challenge = null;
        await refreshCatalog();
        render();
      } else void poll(ticket, nonce);
    } catch (e) {
      if (nonce === generation) {
        challenge = null;
        error = errorMessage(e);
        render();
      }
    }
  }
  element.addEventListener("input", (event) => {
    const input = event.target as HTMLInputElement;
    if (input.dataset.projectName)
      projectNames.set(input.dataset.projectName, input.value);
    switch (input.dataset.field) {
      case "new-space":
        newSpace = input.value;
        break;
      case "label":
        label = input.value;
        break;
      case "identityLabel":
        identityLabel = input.value;
        break;
      case "commitName":
        commitName = input.value;
        break;
      case "commitEmail":
        commitEmail = input.value;
        break;
      case "filter":
        filter = input.value;
        render();
        element
          .querySelector<HTMLInputElement>('[data-field="filter"]')
          ?.focus();
        break;
    }
  });
  element.addEventListener("change", (event) => {
    const input = event.target as HTMLInputElement;
    if (input.dataset.repository) {
      if (input.checked) selected.add(input.dataset.repository);
      else selected.delete(input.dataset.repository);
      render();
    }
    if (input.dataset.field === "existing") existingLogin = input.value;
    if (input.dataset.field === "space") space = input.value;
    if (input.dataset.field === "identity") identity = input.value;
  });
  element.addEventListener("click", async (event) => {
    const action = (event.target as Element).closest<HTMLElement>(
      "[data-setup]",
    )?.dataset.setup;
    if (!action) return;
    event.preventDefault();
    if (action === "stop") {
      cancelled = true;
      render();
      return;
    }
    if (busy) return;
    if (action === "close") {
      generation++;
      if (challenge)
        await setupApi
          .request({ kind: "cancelLogin", id: challenge.id })
          .catch(() => {});
      close();
      return;
    }
    if (action === "cancel-login") {
      generation++;
      const id = challenge?.id;
      challenge = null;
      if (id)
        await work(async () => {
          await setupApi.request({ kind: "cancelLogin", id });
          await refreshCatalog();
        });
      return;
    }
    await work(async () => {
      if (action === "login") {
        const reply = await setupApi.request({ kind: "beginLogin", label });
        if (reply.kind === "login") {
          challenge = reply.value;
          void poll(challenge, ++generation);
          await setupApi.authorize();
        }
      } else if (action === "browser") await setupApi.authorize();
      else if (action === "existing") {
        const reply = await setupApi.request({ kind: "existing" });
        if (reply.kind === "existing") {
          existing = reply.value;
          existingLogin = existing[0] || "";
        }
      } else if (action === "link") {
        const login = existingLogin;
        if (!login) throw new Error("Choose an existing account");
        await setupApi.request({ kind: "link", label, login });
        await refreshCatalog();
      } else if (action.startsWith("disconnect:")) {
        await setupApi.request({
          kind: "disconnect",
          connector: action.split(":")[1],
        });
        await refreshCatalog();
      } else if (action.startsWith("connector:")) {
        connector = action.split(":")[1];
        const reply = await setupApi.request({
          kind: "repositories",
          connector,
        });
        if (reply.kind === "repositories") repositories = reply.value;
        page = "repositories";
      } else if (action === "create-space") {
        const name = newSpace.trim();
        if (!name) throw new Error("Enter a space name");
        const state = await api.mutate(root, {
          kind: "createSpace",
          name,
          path: null,
        });
        spaces = state.spaces;
        space = name;
        newSpace = "";
      } else if (action === "identity") page = "identity";
      else if (action === "use-current") {
        commitName = current[0] || "";
        commitEmail = current[1] || "";
      } else if (action === "add-identity") {
        await setupApi.request({
          kind: "addIdentity",
          label: identityLabel,
          name: commitName,
          email: commitEmail,
        });
        await refreshCatalog();
        identity = catalog.identities.find(
          (i) => i.label === identityLabel,
        )!.id;
      } else if (action === "apply-identity" && project) {
        await setupApi.applyIdentity(
          root,
          project.space,
          project.name,
          identity,
        );
        current = await setupApi.identity(root, project.space, project.name);
      } else if (action === "review") {
        plans = await setupApi.plan(
          root,
          space,
          connector,
          identity,
          [...selected].map((id) => ({
            id,
            project:
              projectNames.get(id) ??
              repositories
                .find((r) => r.id === id)!
                .fullName.split("/")
                .at(-1)!,
          })),
        );
        page = "review";
      } else if (action === "execute") {
        page = "progress";
        cancelled = false;
        render();
        for (const plan of plans) {
          if (cancelled) {
            results.set(plan.id, "Not started");
            continue;
          }
          results.set(plan.id, "Cloning and registering…");
          render();
          try {
            await setupApi.execute(root, plan.id);
            results.set(plan.id, "Ready");
          } catch (e) {
            results.set(plan.id, errorMessage(e));
          }
          render();
        }
      } else if (action === "retry") {
        for (const p of plans)
          if (results.get(p.id) === "Ready")
            selected.delete(
              repositories.find((r) => r.fullName === p.repository)!.id,
            );
        page = "repositories";
      } else if (action === "back")
        page =
          page === "review"
            ? "identity"
            : page === "identity"
              ? "repositories"
              : "connections";
    });
  });
  await work(async () => {
    await refreshCatalog();
    if (project)
      current = await setupApi.identity(root, project.space, project.name);
  });
}
