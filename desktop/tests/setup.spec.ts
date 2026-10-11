import { test, expect, type Page } from "@playwright/test";
async function fixture(page: Page) {
  await page.addInitScript(() => {
    const snapshot = {
      root: "/work/Workspace",
      spaces: [{ name: "personal", path: "/work/Workspace/personal" }],
      projects: [],
      trackedChanges: [],
      activeSpace: null,
      activeProject: null,
    };
    const identities = [
      {
        id: "identity",
        label: "Personal",
        name: "Example",
        email: "example@users.noreply.github.com",
      },
    ];
    const connector = {
      id: "connector",
      label: "Personal GitHub",
      login: "example",
      accountId: "42",
      source: "keychain",
    };
    const state = window as any;
    state.__calls = [];
    state.__failSetup = false;
    state.__emptyRepos = false;
    state.__TAURI_INTERNALS__ = {
      invoke: async (command: string, args: any) => {
        state.__calls.push({ command, args });
        if (command === "open_workspace" || command === "read_workspace")
          return snapshot;
        if (command === "connection_request") {
          const r = args.request;
          if (r.kind === "catalog")
            return {
              kind: "catalog",
              value: { identities, connectors: [connector] },
            };
          if (r.kind === "repositories")
            return {
              kind: "repositories",
              value: state.__emptyRepos
                ? []
                : [
                    { id: "1", fullName: "example/one", private: true },
                    { id: "2", fullName: "example/two", private: false },
                  ],
            };
          if (r.kind === "beginLogin")
            return {
              kind: "login",
              value: {
                id: "login-ticket",
                userCode: "ABCD-EFGH",
                verificationUri: "https://github.com/login/device",
                interval: 10,
              },
            };
          if (r.kind === "pollLogin") return { kind: "connected", value: null };
          if (r.kind === "addIdentity")
            identities.push({
              id: "new-identity",
              label: r.label,
              name: r.name,
              email: r.email,
            });
          return { kind: "done" };
        }
        if (command === "plan_setup")
          return args.repositories.map(
            (selection: { id: string; project: string }) => ({
              id: `plan-${selection.id}`,
              repository: `example/${selection.id === "1" ? "one" : "two"}`,
              project: selection.project,
              path: `/work/Workspace/personal/${selection.project}`,
              connector,
              identity: identities.find((i) => i.id === args.identity) || null,
            }),
          );
        if (
          command === "execute_setup" &&
          state.__failSetup &&
          args.id === "plan-2"
        )
          throw {
            kind: "persistence",
            message: "Checkout retained; register it to recover.",
          };
      },
    };
  });
  await page.goto("/");
  await page.locator("#workspace-path").fill("/work/Workspace");
  await page
    .getByRole("button", { name: "Open workspace", exact: true })
    .click();
  await page.getByRole("button", { name: "Set up from GitHub" }).click();
}
async function choose(page: Page) {
  await page.getByRole("button", { name: "Use connector" }).click();
  await page
    .getByRole("checkbox", { name: "example/one PRIVATE", exact: true })
    .check();
  await page
    .getByRole("checkbox", { name: "example/two PUBLIC", exact: true })
    .check();
  await page.getByRole("button", { name: "Choose identity" }).click();
}
test("review shows separate connector and exact authorship before provisioning multiple repositories", async ({
  page,
}) => {
  await fixture(page);
  await choose(page);
  await page
    .getByLabel("Commit identity", { exact: true })
    .selectOption("identity");
  await page.getByRole("button", { name: "Review setup" }).click();
  await expect(
    page.getByRole("cell", {
      name: "Example <example@users.noreply.github.com>",
      exact: true,
    }),
  ).toHaveCount(2);
  await page.screenshot({
    path: `test-results/setup-review-${test.info().project.name}.png`,
    fullPage: true,
  });
  expect(
    await page.evaluate(() =>
      (window as any).__calls.filter((c: any) => c.command === "execute_setup"),
    ),
  ).toHaveLength(0);
  await page.getByRole("button", { name: "Set up 2 projects" }).click();
  await expect(page.getByText("Ready", { exact: true })).toHaveCount(2);
  const calls = await page.evaluate(() =>
    (window as any).__calls.filter((c: any) => c.command === "execute_setup"),
  );
  expect(calls.map((c: any) => c.args)).toEqual([
    { root: "/work/Workspace", id: "plan-1" },
    { root: "/work/Workspace", id: "plan-2" },
  ]);
});
test("keep Git configuration is explicit and partial setup preserves successful results", async ({
  page,
}) => {
  await fixture(page);
  await page.evaluate(() => {
    (window as any).__failSetup = true;
  });
  await choose(page);
  await page.getByRole("button", { name: "Review setup" }).click();
  const plan = await page.evaluate(() =>
    (window as any).__calls.find((c: any) => c.command === "plan_setup"),
  );
  expect(plan.args.identity).toBe("");
  await page.getByRole("button", { name: "Set up 2 projects" }).click();
  await expect(page.getByText("Ready", { exact: true })).toHaveCount(1);
  await expect(
    page.getByText("Checkout retained; register it to recover.", {
      exact: true,
    }),
  ).toBeVisible();
});
test("device authorization opens a fixed browser destination and can be cancelled", async ({
  page,
}) => {
  await fixture(page);
  await expect(
    page.getByText(/repo scope permits read and write access/),
  ).toBeVisible();
  await expect(
    page.getByText(/No repository installation is needed/),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Sign in with GitHub", exact: true })
    .click();
  await expect(page.getByText("ABCD-EFGH")).toBeVisible();
  await page.getByRole("button", { name: "Cancel sign-in" }).click();
  await expect(page.getByText("ABCD-EFGH")).toHaveCount(0);
  const calls = await page.evaluate(() => (window as any).__calls);
  expect(
    calls.some((c: any) => c.command === "open_github_authorization"),
  ).toBe(true);
  expect(
    calls.some(
      (c: any) =>
        c.command === "connection_request" &&
        c.args.request.kind === "cancelLogin",
    ),
  ).toBe(true);
});

test("local project names can be changed before review", async ({ page }) => {
  await fixture(page);
  await page.getByRole("button", { name: "Use connector" }).click();
  await page
    .getByRole("checkbox", { name: "example/one PRIVATE", exact: true })
    .check();
  await page
    .getByLabel("Local name for example/one", { exact: true })
    .fill("custom-project");
  await page.getByRole("button", { name: "Choose identity" }).click();
  await page.getByRole("button", { name: "Review setup" }).click();
  await expect(
    page.getByRole("cell", {
      name: "/work/Workspace/personal/custom-project",
      exact: true,
    }),
  ).toBeVisible();
  const plan = await page.evaluate(() =>
    (window as any).__calls.find((c: any) => c.command === "plan_setup"),
  );
  expect(plan.args.repositories).toEqual([
    { id: "1", project: "custom-project" },
  ]);
});

test("empty repository list explains OAuth organization approval without an installation request", async ({
  page,
}) => {
  await fixture(page);
  await page.evaluate(() => {
    (window as any).__emptyRepos = true;
  });
  await page
    .getByRole("button", { name: "Use connector", exact: true })
    .click();
  await expect(
    page.getByText(
      "No repositories are visible. Check repository permissions, OAuth App organization approval, and SSO.",
    ),
  ).toBeVisible();
  await expect(page.getByText(/GitHub App installation/)).toHaveCount(0);
});
