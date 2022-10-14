import * as vscode from 'vscode';
import * as child_process from 'child_process';
import { Point } from './proto/sce/Point';
import { Range } from './proto/sce/Range';
import { SliceDirection } from './proto/sce/SliceDirection';
import { Source } from './proto/sce/Source';
import { SliceRequest } from './proto/sce/SliceRequest';
import { SliceResponse } from './proto/sce/SliceResponse';

const ENGINE_BIN = __dirname + "/../../../sce/target/debug/sce";
let engine: child_process.ChildProcess;
let client: SCE;

function removeRanges(src: String, ranges: Range[], srcPoint: vscode.Position): [string, Point] {
    const lines = src.split('\n');
    let newContent = [];
    let newPoint: Point = {line: srcPoint.line, col: srcPoint.character};

    let i = 0;
    for (let { start, end } of ranges) {
        // get around optionality
        start = start as Point;
        end = end as Point;

        if (i < start.line) {
            newContent.push(...lines.slice(i, start.line));
        }
        const prefix = lines[start.line].slice(0, start.col).trim();
        if (prefix.length > 0) {
            newContent.push(prefix);
        }
        const suffix = lines[end.line].slice(end.col).trim();
        if (suffix.length > 0) {
            newContent.push(suffix);
        }
        i = end.line + 1;

        if (end.line < srcPoint.line) {
            let deletedLines = end.line - start.line;
            if (prefix.length === 0 && suffix.length === 0) {
                deletedLines += 1;
            }
            newPoint.line -= deletedLines;
        }
    }

    return [newContent.join('\n'), newPoint];
}

type DisplayHandler = (content: string, point: vscode.Position, language: string, resp: SliceResponse) => void;

async function slice(direction: SliceDirection, displayFunc: DisplayHandler) {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        return;
    }

    const filename = editor.document.fileName;
    const content = editor.document.getText();
    const language = editor.document.languageId;
    const point = editor.selection.active;

    let req: SliceRequest = {
        source: {
            filename,
            content,
            language,
            point: {
                line: point.line,
                col: point.character,
            },
        },
        direction,
    };

    let resp: SliceResponse = await client.slice(req);
    await displayFunc(content, point, language, resp);
}

async function newDocDisplay(content: string, point: vscode.Position, language: string, resp: SliceResponse) {
    const [filtered, targetPoint] = removeRanges(content, resp.toRemove, point);

    const sliceDoc = await vscode.workspace.openTextDocument({
        language: language,
        content: filtered,
    });
    // open the document as a preview
    await vscode.window.showTextDocument(sliceDoc, {
        preview: true,
        selection: new vscode.Range(targetPoint.line, targetPoint.col, targetPoint.line, targetPoint.col),
        viewColumn: vscode.ViewColumn.Beside,
    });
}

async function foldDisplay(content: string, point: vscode.Position, language: string, resp: SliceResponse) {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        return;
    }

    const ranges = resp.toRemove.map((range: Range) => {
        return new vscode.Range(range.start?.line || 0, range.start?.col || 0, range.end?.line || 0, range.end?.col || 0);
    });
    editor.selections = ranges.map((range: vscode.Range) => {
        return new vscode.Selection(range.start, range.end);
    });
    await vscode.commands.executeCommand('editor.createFoldingRangeFromSelection');
}

async function inline() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        return;
    }
}

export function activate(context: vscode.ExtensionContext) {
    engine = child_process.spawn(ENGINE_BIN);
    engine.on('error', (err) => {
        vscode.window.showErrorMessage("Error starting SCE: " + (err as Error).toString());
    }).on('exit', (code) => {
        if (code !== 0) {
            vscode.window.showErrorMessage(`SCE exited with code ${code}`);
        }
    }).on('close', (code) => {
        if (code !== 0) {
            vscode.window.showErrorMessage(`SCE exited with code ${code}`);
        }
    });
    client = new SCEClient('localhost:1486', credentials.createInsecure());

	context.subscriptions.push(vscode.commands.registerCommand('sce.sliceBackwardW', async () => {
        await slice(SliceDirection.BACKWARD, newDocDisplay);
	}));
	context.subscriptions.push(vscode.commands.registerCommand('sce.sliceForwardW', async () => {
        await slice(SliceDirection.FORWARD, newDocDisplay);
	}));
	context.subscriptions.push(vscode.commands.registerCommand('sce.sliceBackwardF', async () => {
        await slice(SliceDirection.BACKWARD, foldDisplay);
	}));
	context.subscriptions.push(vscode.commands.registerCommand('sce.sliceForwardF', async () => {
        await slice(SliceDirection.FORWARD, foldDisplay);
	}));
	context.subscriptions.push(vscode.commands.registerCommand('sce.inline', async () => {
        await inline();
	}));
}

export function deactivate() {
    engine.kill();
}