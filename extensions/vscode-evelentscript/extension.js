'use strict';

const vscode = require('vscode');
const path = require('path');
const ts = require('typescript');
const { EvelentLanguageService } = require('./language-service/ts-service');
const { getKeywordCompletions, getWordPrefix } = require('./language-service/keywords');

const LANGUAGE_IDS = ['evelentscript', 'literate-evelentscript'];
const DEBOUNCE_MS = 250;

/** @type {vscode.OutputChannel | undefined} */
let outputChannel;
/** @type {vscode.DiagnosticCollection | undefined} */
let diagnostics;
/** @type {Map<string, EvelentLanguageService>} */
const services = new Map();
/** @type {Map<string, NodeJS.Timeout>} */
const pendingUpdates = new Map();

function log(message) {
  outputChannel?.appendLine(message);
}

function getWorkspaceRoot(document) {
  const folder = vscode.workspace.getWorkspaceFolder(document.uri);
  if (folder) {
    return folder.uri.fsPath;
  }
  return path.dirname(document.uri.fsPath);
}

function getService(document) {
  const root = getWorkspaceRoot(document);
  if (!services.has(root)) {
    services.set(root, new EvelentLanguageService(root, path.join(__dirname)));
  }
  return services.get(root);
}

function isEnabled() {
  return vscode.workspace.getConfiguration('evelentscript').get('intellisense.enable', true);
}

function isDiagnosticsEnabled() {
  return vscode.workspace.getConfiguration('evelentscript').get('diagnostics.enable', true);
}

function isSemanticDiagnosticsEnabled() {
  return vscode.workspace.getConfiguration('evelentscript').get('diagnostics.semantic', true);
}

async function publishDiagnostics(document) {
  if (!diagnostics) {
    return;
  }
  if (!isEnabled() || !isDiagnosticsEnabled()) {
    diagnostics.delete(document.uri);
    return;
  }
  try {
    const service = getService(document);
    const results = await service.getDiagnostics(document.uri.fsPath, {
      semantic: isSemanticDiagnosticsEnabled(),
    });
    const items = results.map((d) => {
      const range = new vscode.Range(
        new vscode.Position(Math.max(0, d.startLine), Math.max(0, d.startColumn)),
        new vscode.Position(Math.max(0, d.endLine), Math.max(0, d.endColumn))
      );
      const diagnostic = new vscode.Diagnostic(
        range,
        d.message,
        d.severity === 'warning'
          ? vscode.DiagnosticSeverity.Warning
          : vscode.DiagnosticSeverity.Error
      );
      diagnostic.source = 'evelentscript';
      return diagnostic;
    });
    diagnostics.set(document.uri, items);
  } catch (error) {
    log(`diagnostics failed: ${error.message}`);
  }
}

function scheduleUpdate(document) {
  const key = document.uri.toString();
  if (pendingUpdates.has(key)) {
    clearTimeout(pendingUpdates.get(key));
  }
  pendingUpdates.set(
    key,
    setTimeout(() => {
      pendingUpdates.delete(key);
      try {
        getService(document).updateDocument(document.uri.fsPath, document.getText());
      } catch (error) {
        log(`IntelliSense update failed: ${error.message}`);
      }
      void publishDiagnostics(document);
    }, DEBOUNCE_MS)
  );
}

function getReplaceRange(document, position) {
  const lineText = document.lineAt(position.line).text;
  const prefix = getWordPrefix(lineText, position.character);
  if (!prefix) {
    return undefined;
  }
  const start = new vscode.Position(position.line, position.character - prefix.length);
  return new vscode.Range(start, position);
}

function tsKindToCompletionKind(kind) {
  switch (kind) {
    case 'keyword':
      return vscode.CompletionItemKind.Keyword;
    case ts.ScriptElementKind.memberVariableElement:
    case ts.ScriptElementKind.memberFunctionElement:
      return vscode.CompletionItemKind.Field;
    case ts.ScriptElementKind.variableElement:
      return vscode.CompletionItemKind.Variable;
    case ts.ScriptElementKind.functionElement:
    case ts.ScriptElementKind.localFunctionElement:
      return vscode.CompletionItemKind.Function;
    case ts.ScriptElementKind.classElement:
      return vscode.CompletionItemKind.Class;
    case ts.ScriptElementKind.interfaceElement:
      return vscode.CompletionItemKind.Interface;
    case ts.ScriptElementKind.typeElement:
      return vscode.CompletionItemKind.TypeParameter;
    case ts.ScriptElementKind.enumElement:
      return vscode.CompletionItemKind.Enum;
    case ts.ScriptElementKind.moduleElement:
    case ts.ScriptElementKind.externalModuleName:
      return vscode.CompletionItemKind.Module;
    case ts.ScriptElementKind.keyword:
      return vscode.CompletionItemKind.Keyword;
    case ts.ScriptElementKind.constElement:
      return vscode.CompletionItemKind.Constant;
    case ts.ScriptElementKind.letElement:
      return vscode.CompletionItemKind.Variable;
    case ts.ScriptElementKind.methodElement:
      return vscode.CompletionItemKind.Method;
    case ts.ScriptElementKind.constructorImplementationElement:
      return vscode.CompletionItemKind.Constructor;
    case ts.ScriptElementKind.alias:
      return vscode.CompletionItemKind.Reference;
    case 'property':
      return vscode.CompletionItemKind.Property;
    case 'external module name':
      return vscode.CompletionItemKind.Module;
    default:
      return vscode.CompletionItemKind.Variable;
  }
}

function entriesToCompletionList(document, position, entries) {
  const replaceRange = getReplaceRange(document, position);
  const items = entries.map((entry) => {
    const item = new vscode.CompletionItem(entry.name, tsKindToCompletionKind(entry.kind));
    item.sortText = entry.sortText || `!1${entry.name}`;
    item.preselect = entry.isRecommended;
    if (replaceRange) {
      item.range = replaceRange;
    }
    return item;
  });
  return new vscode.CompletionList(items, false);
}

function keywordFallback(document, position) {
  const lineText = document.lineAt(position.line).text;
  const entries = getKeywordCompletions(lineText, position.character);
  if (!entries.length) {
    return undefined;
  }
  return entriesToCompletionList(document, position, entries);
}

function displayPartsToMarkdown(parts) {
  if (!parts?.length) {
    return undefined;
  }
  return parts.map((part) => part.text).join('');
}

function activate(context) {
  outputChannel = vscode.window.createOutputChannel('EvelentScript');
  context.subscriptions.push(outputChannel);
  diagnostics = vscode.languages.createDiagnosticCollection('evelentscript');
  context.subscriptions.push(diagnostics);
  log('Extension activated');

  try {
    require(path.join(__dirname, 'bundled', 'evelentscript'));
    log('Compiler: OK (bundled)');
  } catch (error) {
    log(`Compiler: MISSING — ${error.message}`);
    void vscode.window.showWarningMessage(
      'EvelentScript: compiler not bundled. Reinstall the VSIX built with build.bat.'
    );
  }

  const iconTheme = vscode.workspace.getConfiguration('workbench').get('iconTheme');
  if (iconTheme === 'evelent-icons') {
    void vscode.workspace
      .getConfiguration('workbench')
      .update('iconTheme', undefined, vscode.ConfigurationTarget.Workspace);
    void vscode.window.showInformationMessage(
      'EvelentScript больше не меняет icon theme. Выбери снова Catppuccin: File Icon Theme.',
      'Выбрать тему'
    ).then((choice) => {
      if (choice) {
        void vscode.commands.executeCommand('workbench.action.selectIconTheme');
      }
    });
  }

  for (const languageId of LANGUAGE_IDS) {
    context.subscriptions.push(
      vscode.workspace.onDidOpenTextDocument((doc) => {
        if (doc.languageId === languageId && isEnabled()) {
          scheduleUpdate(doc);
        }
      }),
      vscode.workspace.onDidChangeTextDocument((event) => {
        if (event.document.languageId === languageId && isEnabled()) {
          scheduleUpdate(event.document);
        }
      }),
      vscode.workspace.onDidCloseTextDocument((doc) => {
        if (doc.languageId !== languageId) {
          return;
        }
        const key = doc.uri.toString();
        if (pendingUpdates.has(key)) {
          clearTimeout(pendingUpdates.get(key));
          pendingUpdates.delete(key);
        }
        try {
          getService(doc).removeDocument(doc.uri.fsPath);
        } catch (_) {
          // ignore
        }
        diagnostics?.delete(doc.uri);
      })
    );
  }

  for (const languageId of LANGUAGE_IDS) {
    context.subscriptions.push(
      vscode.languages.registerCompletionItemProvider(
        { language: languageId },
        {
          async provideCompletionItems(document, position, token) {
            if (!isEnabled() || token.isCancellationRequested) {
              return undefined;
            }
            const lineText = document.lineAt(position.line).text;
            try {
              const service = getService(document);
              service.updateDocument(document.uri.fsPath, document.getText());
              const result = await service.getCompletions(
                document.uri.fsPath,
                position.line + 1,
                position.character,
                lineText
              );
              if (result?.entries?.length) {
                return entriesToCompletionList(document, position, result.entries);
              }
            } catch (error) {
              log(`completion failed: ${error.message}`);
            }
            return keywordFallback(document, position);
          },
        },
        '.',
        '"',
        "'",
        '/',
        '<',
        '@',
        '#',
        ' '
      ),

      vscode.languages.registerHoverProvider(languageId, {
        async provideHover(document, position, token) {
          if (!isEnabled() || token.isCancellationRequested) {
            return undefined;
          }
          try {
            const service = getService(document);
            service.updateDocument(document.uri.fsPath, document.getText());
            const info = await service.getQuickInfo(
              document.uri.fsPath,
              position.line + 1,
              position.character
            );
            if (!info) {
              return undefined;
            }
            const signature = displayPartsToMarkdown(info.displayParts);
            const docs = displayPartsToMarkdown(info.documentation);
            const contents = [];
            if (signature) {
              contents.push(new vscode.MarkdownString('```typescript\n' + signature + '\n```'));
            }
            if (docs) {
              contents.push(new vscode.MarkdownString(docs));
            }
            return contents.length ? new vscode.Hover(contents) : undefined;
          } catch (error) {
            log(`hover failed: ${error.message}`);
            return undefined;
          }
        },
      }),

      vscode.languages.registerDefinitionProvider(languageId, {
        async provideDefinition(document, position, token) {
          if (!isEnabled() || token.isCancellationRequested) {
            return undefined;
          }
          try {
            const service = getService(document);
            service.updateDocument(document.uri.fsPath, document.getText());
            const result = await service.getDefinition(
              document.uri.fsPath,
              position.line + 1,
              position.character
            );
            if (!result?.definitions?.length) {
              return undefined;
            }
            return result.definitions
              .filter((def) => def.fileName && def.textSpan)
              .map((def) => {
                const esPath = def.fileName.replace(/\.evelent\.ts$/, '');
                const start = positionFromOffset(document, def.textSpan.start, esPath !== def.fileName);
                const end = positionFromOffset(
                  document,
                  def.textSpan.start + def.textSpan.length,
                  esPath !== def.fileName
                );
                return new vscode.Location(vscode.Uri.file(esPath), new vscode.Range(start, end));
              });
          } catch (error) {
            log(`definition failed: ${error.message}`);
            return undefined;
          }
        },
      }),

      vscode.languages.registerSignatureHelpProvider(
        languageId,
        {
          async provideSignatureHelp(document, position, token) {
            if (!isEnabled() || token.isCancellationRequested) {
              return undefined;
            }
            try {
              const service = getService(document);
              service.updateDocument(document.uri.fsPath, document.getText());
              const help = await service.getSignatureHelp(
                document.uri.fsPath,
                position.line + 1,
                position.character
              );
              if (!help?.items?.length) {
                return undefined;
              }
              const signatures = help.items.map((item) => {
                const signature = new vscode.SignatureInformation(
                  displayPartsToMarkdown(item.prefixDisplayParts) || ''
                );
                signature.parameters = (item.parameters || []).map((param) => {
                  const label = displayPartsToMarkdown(param.displayParts) || param.name;
                  return new vscode.ParameterInformation(label);
                });
                return signature;
              });
              return new vscode.SignatureHelp(signatures, help.selectedItemIndex, help.argumentIndex);
            } catch (error) {
              log(`signature help failed: ${error.message}`);
              return undefined;
            }
          },
        },
        '(',
        ','
      )
    );
  }

  for (const document of vscode.workspace.textDocuments) {
    if (LANGUAGE_IDS.includes(document.languageId) && isEnabled()) {
      scheduleUpdate(document);
    }
  }

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (!event.affectsConfiguration('evelentscript')) {
        return;
      }
      for (const document of vscode.workspace.textDocuments) {
        if (LANGUAGE_IDS.includes(document.languageId)) {
          void publishDiagnostics(document);
        }
      }
    })
  );
}

function positionFromOffset(document, offset, useDocument) {
  if (useDocument) {
    return document.positionAt(offset);
  }
  const text = document.getText();
  let line = 0;
  let column = 0;
  for (let i = 0; i < offset && i < text.length; i++) {
    if (text[i] === '\n') {
      line++;
      column = 0;
    } else {
      column++;
    }
  }
  return new vscode.Position(line, column);
}

function deactivate() {
  for (const timer of pendingUpdates.values()) {
    clearTimeout(timer);
  }
  pendingUpdates.clear();
  services.clear();
  diagnostics?.clear();
}

module.exports = {
  activate,
  deactivate,
};
