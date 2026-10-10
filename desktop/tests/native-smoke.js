// Executed in the real Tauri webview only with the native-smoke Cargo feature.
(async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const waitFor = async (predicate) => {
    const deadline = Date.now() + 20000;
    while (!predicate()) {
      if (Date.now() > deadline)
        throw new Error("Timed out: " + document.body.innerText);
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  };
  const click = (selector) => {
    const element = document.querySelector(selector);
    if (!element || element.disabled)
      throw new Error("Unavailable control: " + selector);
    element.click();
  };
  const fill = (selector, value) => {
    const input = document.querySelector(selector);
    input.value = value;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  };
  const submit = () =>
    document.querySelector("#operation-form").requestSubmit();
  const idle = () =>
    document.querySelector("#app").getAttribute("aria-busy") === "false";
  try {
    await waitFor(() => document.querySelector("#workspace-path"));
    fill("#workspace-path", SMOKE_ROOT);
    click('[data-action="initialize"]');
    await waitFor(() => idle() && document.querySelector(".workspace-main"));
    click('[data-action="new-space"]');
    fill("#field-name", "team");
    submit();
    await waitFor(() => idle() && !document.querySelector("dialog"));
    click('[data-action="register"]');
    fill("#field-name", "app");
    fill("#field-path", SMOKE_ROOT + "/existing");
    submit();
    await waitFor(() => idle() && document.querySelector(".project-select"));
    click(".project-select");
    click('[data-action="new-change"]');
    fill("#field-name", "feat/native");
    submit();
    await waitFor(
      () => idle() && document.querySelector('[data-action="finish"]'),
    );
    click('[data-action="finish"]');
    submit();
    await waitFor(
      () =>
        idle() &&
        !document.querySelector("dialog") &&
        !document.querySelector('[data-action="finish"]'),
    );
    click('[data-action="unregister-project"]');
    submit();
    await waitFor(
      () =>
        idle() &&
        !document.querySelector("dialog") &&
        !document.querySelector(".project-card"),
    );
    await invoke("plugin:event|emit", {
      event: "spacer-native-smoke",
      payload: { success: true },
    });
  } catch (error) {
    await invoke("plugin:event|emit", {
      event: "spacer-native-smoke",
      payload: { success: false, error: String(error) },
    });
  }
})();
