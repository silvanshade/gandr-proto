"use strict";
var __createBinding =
  (this && this.__createBinding) ||
  (Object.create
    ? function (o, m, k, k2) {
        if (k2 === undefined) k2 = k;
        var desc = Object.getOwnPropertyDescriptor(m, k);
        if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
          desc = {
            enumerable: true,
            get: function () {
              return m[k];
            },
          };
        }
        Object.defineProperty(o, k2, desc);
      }
    : function (o, m, k, k2) {
        if (k2 === undefined) k2 = k;
        o[k2] = m[k];
      });
var __setModuleDefault =
  (this && this.__setModuleDefault) ||
  (Object.create
    ? function (o, v) {
        Object.defineProperty(o, "default", { enumerable: true, value: v });
      }
    : function (o, v) {
        o["default"] = v;
      });
var __importStar =
  (this && this.__importStar) ||
  (function () {
    var ownKeys = function (o) {
      ownKeys =
        Object.getOwnPropertyNames ||
        function (o) {
          var ar = [];
          for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
          return ar;
        };
      return ownKeys(o);
    };
    return function (mod) {
      if (mod && mod.__esModule) return mod;
      var result = {};
      if (mod != null)
        for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
      __setModuleDefault(result, mod);
      return result;
    };
  })();
Object.defineProperty(exports, "__esModule", { value: true });
exports.GandrLanguageClient = void 0;
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
class GandrLanguageClient extends node_1.LanguageClient {
  constructor(id, name, serverOptions, clientOptions, forceDebug) {
    super(id, name, serverOptions, clientOptions, forceDebug);
  }
  static create() {
    const outputChannel = vscode.window.createOutputChannel("gandr language server", {
      log: true,
    });
    const traceOutputChannel = vscode.window.createOutputChannel("gandr language server trace", { log: true });
    const serverOptions = {
      run: {
        command: "gandr",
        args: ["lsp"],
      },
      debug: {
        command: "gandr",
        args: ["lsp"],
      },
    };
    const clientOptions = {
      documentSelector: [{ scheme: "file", language: "gandr" }],
      outputChannel,
      traceOutputChannel,
      synchronize: {
        fileEvents: vscode.workspace.createFileSystemWatcher("**/*.gandr"),
      },
    };
    const client = new this("gandr", "gandr language server", serverOptions, clientOptions);
    client.registerProposedFeatures();
    return client;
  }
  async start() {
    await super.start();
  }
}
exports.GandrLanguageClient = GandrLanguageClient;
