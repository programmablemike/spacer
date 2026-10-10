import { test, expect, type Page } from "@playwright/test";

async function fixture(page: Page) {
  await page.addInitScript(() => {
    const data = {
      root: "/work/Workshop",
      spaces: [
        { name: "studio", path: "/work/Workshop/studio" },
        { name: "personal", path: "/work/Workshop/personal" },
      ],
      projects: [
        {
          name: "spacer",
          space: "studio",
          path: "/work/Workshop/studio/spacer",
          error: null,
          worktrees: [
            {
              branch: "main",
              path: "/work/Workshop/studio/spacer",
              isPrimary: true,
            },
            {
              branch: "feat/desktop",
              path: "/work/Workshop/.spacer/worktrees/studio/spacer/feat/desktop",
              isPrimary: false,
            },
            {
              branch: null,
              path: "/work/experiments/detached",
              isPrimary: false,
            },
          ],
        },
        {
          name: "field-notes",
          space: "personal",
          path: "/work/Workshop/personal/field-notes",
          error: null,
          worktrees: [
            {
              branch: "main",
              path: "/work/Workshop/personal/field-notes",
              isPrimary: true,
            },
          ],
        },
      ],
      trackedChanges: [
        { name: "old-prototype", space: "studio", project: "spacer" },
      ],
      activeSpace: "studio",
      activeProject: "spacer",
    };
    const state = window as unknown as {
      __calls: { command: string; args: any }[];
      __fixture: typeof data;
      __failFinish: boolean;
      __failMutation: boolean;
      __TAURI_INTERNALS__: {
        invoke: (command: string, args: any) => Promise<any>;
      };
    };
    state.__calls = [];
    state.__fixture = data;
    state.__failFinish = true;
    state.__failMutation = false;
    state.__TAURI_INTERNALS__ = {
      invoke: async (command, args) => {
        state.__calls.push({ command, args });
        if (command === "plugin:dialog|open") return "/work/Workshop";
        if (command === "open_workspace") {
          if (args.root.includes("missing"))
            throw {
              kind: "invalidOperation",
              message: "This folder is not an initialized workspace.",
            };
          return structuredClone(data);
        }
        if (command === "read_workspace") return structuredClone(data);
        if (command === "resolve_target")
          return args.target.path || "/work/Workshop/studio/spacer";
        if (
          command === "reveal_target" ||
          command === "plugin:clipboard-manager|write_text"
        )
          return;
        if (command === "mutate_workspace") {
          await new Promise((resolve) => setTimeout(resolve, 100));
          const op = args.operation;
          if (state.__failMutation)
            throw {
              kind: "persistence",
              message: "Configuration changed; refresh before retrying.",
            };
          if (op.kind === "finishChange") {
            if (state.__failFinish)
              throw {
                kind: "invalidOperation",
                message:
                  "Worktree contains modified or untracked files; preserve them before finishing.",
              };
            data.projects[0].worktrees = data.projects[0].worktrees.filter(
              (tree) => tree.branch !== op.name,
            );
          } else if (op.kind === "createSpace")
            data.spaces.push({
              name: op.name,
              path: "/work/Workshop/" + op.name,
            });
          else if (op.kind === "createProject" || op.kind === "registerProject")
            data.projects.push({
              name: op.name,
              space: op.space,
              path: op.path || "/work/Workshop/" + op.space + "/" + op.name,
              error: null,
              worktrees: [],
            });
          else if (op.kind === "startChange")
            data.projects[0].worktrees.push({
              branch: op.name,
              path: "/work/changes/" + op.name,
              isPrimary: false,
            });
          else if (op.kind === "unregisterProject")
            data.projects = data.projects.filter(
              (p) => p.name !== op.project || p.space !== op.space,
            );
          else if (op.kind === "removeTrackedChange") data.trackedChanges = [];
          return structuredClone(data);
        }
        throw new Error("Unexpected IPC command: " + command);
      },
    };
  });
  await page.goto("/");
}
async function openWorkspace(page: Page) {
  await page.getByRole("button", { name: "Browse…", exact: true }).click();
  await expect(page.getByLabel("Workspace folder")).toHaveValue(
    "/work/Workshop",
  );
  await page
    .getByRole("button", { name: "Open workspace", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Workspace overview" }),
  ).toBeVisible();
}
async function openProject(page: Page) {
  await openWorkspace(page);
  await page
    .locator(".project-select")
    .filter({ has: page.getByRole("heading", { name: "spacer", exact: true }) })
    .click();
  await expect(
    page.getByRole("heading", { name: "spacer", exact: true, level: 1 }),
  ).toBeVisible();
}

test("opens through native picker, filters projects, and refreshes external changes", async ({
  page,
}) => {
  await fixture(page);
  await openWorkspace(page);
  await page.getByRole("textbox", { name: "Filter projects" }).fill("field");
  await expect(page.locator(".project-card")).toHaveCount(1);
  await expect(
    page.getByRole("heading", { name: "field-notes" }),
  ).toBeVisible();
  await page.getByRole("textbox", { name: "Filter projects" }).fill("");
  await page.evaluate(() => {
    (window as any).__fixture.projects[1].name = "updated-outside";
  });
  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "updated-outside" }),
  ).toBeVisible();
});

test("initialization is explicit and open errors keep the chosen path", async ({
  page,
}) => {
  await fixture(page);
  await page.getByLabel("Workspace folder").fill("/work/missing");
  await page
    .getByRole("button", { name: "Open workspace", exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText(
    "not an initialized workspace",
  );
  await expect(page.getByLabel("Workspace folder")).toHaveValue(
    "/work/missing",
  );
  await page.getByLabel("Workspace folder").fill("/work/Workshop");
  await page
    .getByRole("button", { name: "Create workspace", exact: true })
    .click();
  const calls = await page.evaluate(() =>
    (window as any).__calls.filter(
      (call: any) => call.command === "open_workspace",
    ),
  );
  expect(calls.map((call: any) => call.args.initialize)).toEqual([false, true]);
});

test("creates and registers through typed operations; failed forms preserve input", async ({
  page,
}) => {
  await fixture(page);
  await openWorkspace(page);
  await page.getByRole("button", { name: "New project", exact: true }).click();
  await page.getByLabel("Name", { exact: true }).fill("new-project");
  await page.evaluate(() => {
    (window as any).__failMutation = true;
  });
  await page
    .getByRole("button", { name: "Create project", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toContainText("Configuration changed");
  await expect(page.getByLabel("Name", { exact: true })).toHaveValue(
    "new-project",
  );
  await page.evaluate(() => {
    (window as any).__failMutation = false;
  });
  await page
    .getByRole("button", { name: "Create project", exact: true })
    .click();
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await expect(
    page.getByRole("heading", { name: "new-project" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Register project", exact: true })
    .click();
  await page.getByLabel("Name", { exact: true }).fill("existing");
  await page.getByLabel("Repository folder").fill("/external/repo with spaces");
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Register project", exact: true })
    .click();
  const operation = await page.evaluate(
    () =>
      (window as any).__calls
        .filter((call: any) => call.command === "mutate_workspace")
        .at(-1).args.operation,
  );
  expect(operation).toEqual({
    kind: "registerProject",
    name: "existing",
    space: "studio",
    path: "/external/repo with spaces",
  });
});

test("finish requires review; dirty refusal preserves the worktree and offers retry", async ({
  page,
}) => {
  await fixture(page);
  await openProject(page);
  await page.getByRole("button", { name: "Finish", exact: true }).click();
  await expect(page.getByRole("dialog")).toContainText(
    "branch and all commits are retained",
  );
  expect(
    await page.evaluate(() =>
      (window as any).__calls.some(
        (call: any) => call.command === "mutate_workspace",
      ),
    ),
  ).toBe(false);
  await page
    .getByRole("button", { name: "Finish change", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toContainText(
    "modified or untracked files",
  );
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "feat/desktop", exact: true }),
  ).toBeVisible();
  await page.evaluate(() => {
    (window as any).__failFinish = false;
  });
  await page.getByRole("button", { name: "Finish", exact: true }).click();
  await page
    .getByRole("button", { name: "Finish change", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "feat/desktop", exact: true }),
  ).not.toBeVisible();
  await expect(page.getByRole("status")).toContainText(
    "branch and commits are retained",
  );
});

test("detached worktrees open and copy their actual path without a finish action", async ({
  page,
}) => {
  await fixture(page);
  await openProject(page);
  const row = page
    .locator(".worktree")
    .filter({ hasText: "Detached checkout" });
  await expect(
    row.getByRole("button", { name: "Finish", exact: true }),
  ).toHaveCount(0);
  await row.getByRole("button", { name: "Open", exact: true }).click();
  await row.getByRole("button", { name: "Copy path", exact: true }).click();
  const calls = await page.evaluate(() => (window as any).__calls);
  expect(
    calls.find((call: any) => call.command === "reveal_target").args.target,
  ).toEqual({
    kind: "worktree",
    space: "studio",
    project: "spacer",
    path: "/work/experiments/detached",
  });
  expect(
    calls.find(
      (call: any) => call.command === "plugin:clipboard-manager|write_text",
    ).args.text,
  ).toBe("/work/experiments/detached");
});

test("unregister wording preserves files and cancel sends no mutation", async ({
  page,
}) => {
  await fixture(page);
  await openProject(page);
  await page
    .getByRole("button", { name: "Unregister project", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toContainText(
    "files, branches, and worktrees are preserved",
  );
  await page.keyboard.press("Escape");
  expect(
    await page.evaluate(() =>
      (window as any).__calls.some(
        (call: any) => call.command === "mutate_workspace",
      ),
    ),
  ).toBe(false);
  await page
    .getByRole("button", { name: "Unregister project", exact: true })
    .click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Unregister project", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "spacer", exact: true }),
  ).toHaveCount(0);
  await expect(page.getByRole("status")).toContainText(
    "All files are preserved",
  );
});

test("repository errors stay visible and external text is escaped", async ({
  page,
}) => {
  await fixture(page);
  await page.evaluate(() => {
    (window as any).__fixture.projects[0].error = {
      kind: "git",
      message: "Repository is missing <img src=x onerror=alert(1)>",
    };
  });
  await openProject(page);
  await expect(page.getByRole("alert")).toContainText(
    "Repository is missing <img",
  );
  await expect(page.locator("img[src=x]")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Unregister project", exact: true }),
  ).toBeEnabled();
});

test("layout fits the minimum window and captures the desktop views", async ({
  page,
}, testInfo) => {
  await fixture(page);
  await page.screenshot({ path: testInfo.outputPath("welcome.png") });
  await openWorkspace(page);
  await page.screenshot({ path: testInfo.outputPath("overview.png") });
  await page.locator(".project-select").filter({ hasText: "spacer" }).click();
  await page.screenshot({ path: testInfo.outputPath("project.png") });
  await page.setViewportSize({ width: 780, height: 560 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await expect(
    page.getByRole("button", { name: "Start change", exact: true }),
  ).toBeVisible();
});

test("creates spaces and changes, saves defaults, and removes legacy tracking explicitly", async ({
  page,
}) => {
  await fixture(page);
  await openWorkspace(page);
  await page.getByRole("button", { name: "Create space", exact: true }).click();
  await page.getByLabel("Name", { exact: true }).fill("new-team");
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Create space", exact: true })
    .click();
  await expect(
    page.locator("nav").getByRole("button", { name: /new-team/ }),
  ).toBeVisible();
  await page.locator(".project-select").filter({ hasText: "spacer" }).click();
  await page.getByRole("button", { name: "Start change", exact: true }).click();
  await page.getByLabel("Branch name").fill("feat/new-window");
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Start change", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "feat/new-window", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Default project", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Remove tracking", exact: true })
    .click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Remove tracking", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Legacy tracking", exact: true }),
  ).toHaveCount(0);
  const operations = await page.evaluate(() =>
    (window as any).__calls
      .filter((call: any) => call.command === "mutate_workspace")
      .map((call: any) => call.args.operation),
  );
  expect(operations).toEqual([
    { kind: "createSpace", name: "new-team", path: null },
    {
      kind: "startChange",
      name: "feat/new-window",
      project: "spacer",
      space: "studio",
    },
    { kind: "useProject", project: "spacer", space: "studio" },
    {
      kind: "removeTrackedChange",
      name: "old-prototype",
      project: "spacer",
      space: "studio",
    },
  ]);
});
