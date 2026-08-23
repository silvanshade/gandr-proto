import * as vscode from "vscode";

import { GandrLanguageClient } from "./lsp/client";

export async function activate(context: vscode.ExtensionContext) {
  const languageClient = GandrLanguageClient.create();
  await languageClient.start();
  context.subscriptions.push(languageClient);
}

export function deactivate() {}
