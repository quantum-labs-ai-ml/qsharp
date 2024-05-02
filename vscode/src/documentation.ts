// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

import { type Circuit as CircuitData } from "@microsoft/quantum-viz.js/lib";
import { escapeHtml } from "markdown-it/lib/common/utils";
import {
  ICompilerWorker,
  IOperationInfo,
  IRange,
  ProgramConfig,
  TargetProfile,
  VSDiagnostic,
  getCompilerWorker,
  log,
} from "qsharp-lang";
import { Uri, window } from "vscode";
import { basename, isQsharpDocument } from "./common";
import { getTarget, getTargetFriendlyName } from "./config";
import { loadProject } from "./projectSystem";
import { EventType, UserFlowStatus, sendTelemetryEvent } from "./telemetry";
import { getRandomGuid } from "./utils";
import { sendMessageToPanel } from "./webviewPanel";

export async function showDocumentationCommand(
    extensionUri: Uri,
    operation: IOperationInfo | undefined,
  ) {
    sendMessageToPanel(
      "documentationPanelType", // This is needed to route the message to the proper panel
      true,
      null);

    const editor = window.activeTextEditor;
    if (!editor || !isQsharpDocument(editor.document)) {
      throw new Error("The currently active window is not a Q# file");
    }

    const docUri = editor.document.uri;
    const program = await loadProject(docUri);
    const targetProfile = getTarget();
    const programPath = docUri.path;

    const compilerWorkerScriptPath = Uri.joinPath(
      extensionUri,
      "./out/compilerWorker.js",
    ).toString();
    const worker = getCompilerWorker(compilerWorkerScriptPath);
    const docFiles = await worker.getDocumentation(program, targetProfile);

    var content: string = "";
    for (const a of docFiles ) {
        content += a.filename + "\n\n";
    }

    const message = {
      command: "showDocumentationCommand", // This is handled in webview.tsx onMessage
      contentToRender: content,
    };

    sendMessageToPanel(
      "documentationPanelType", // This is needed to route the message to the proper panel
      true,
      message);
  }

