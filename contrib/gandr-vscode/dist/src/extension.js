"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const client_1 = require("./lsp/client");
async function activate(context) {
  const languageClient = client_1.GandrLanguageClient.create();
  await languageClient.start();
  context.subscriptions.push(languageClient);
}
function deactivate() {}
