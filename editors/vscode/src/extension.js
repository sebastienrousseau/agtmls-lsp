// SPDX-FileCopyrightText: 2026 Sebastien Rousseau
// SPDX-License-Identifier: Apache-2.0 OR MIT

/**
 * VS Code client for agtmls-lsp.
 *
 * Deliberately thin. Every rule, every code action and every decision about
 * severity lives in the server, which is the same engine the CLI runs — an
 * editor that reimplements any of that is an editor that will eventually
 * disagree with CI, and the author will believe the editor.
 */

const { workspace, window } = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;

function activate(context) {
  const config = workspace.getConfiguration("agtmls");
  const command = config.get("serverPath") || "agtmls-lsp";
  const specPath = config.get("specPath");

  const env = { ...process.env };
  if (specPath) env.AGTMLS_SPEC = specPath;

  const serverOptions = {
    run: { command, transport: TransportKind.stdio, options: { env } },
    debug: { command, transport: TransportKind.stdio, options: { env } },
  };

  client = new LanguageClient(
    "agtmls",
    "AgtMLS",
    serverOptions,
    {
      documentSelector: [{ scheme: "file", language: "markdown" }],
      synchronize: { fileEvents: workspace.createFileSystemWatcher("**/metadata.json") },
    },
  );

  client.start().catch((error) => {
    // The common failure is the binary not being installed, and "command
    // failed" does not tell anyone that.
    window.showErrorMessage(
      `AgtMLS: could not start ${command}. Install it with \`cargo install agtmls-lsp\`, ` +
        `or set agtmls.serverPath. (${error?.message ?? error})`,
    );
  });

  context.subscriptions.push({ dispose: () => client?.stop() });
}

function deactivate() {
  return client?.stop();
}

module.exports = { activate, deactivate };
