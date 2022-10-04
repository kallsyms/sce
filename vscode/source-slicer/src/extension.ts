import * as vscode from 'vscode';
import * as child_process from 'child_process';

const SLICER_BIN = __dirname + "/../../../slicer/target/debug/slicer";

type SliceDirection = "Backward" | "Forward";

type SlicerRequest = {
    direction: SliceDirection;
};

type InlineRequest = {
    target_content: string;
    target_point: [number, number];
};

type Request = {
    filename: string;
    language: string;
    content: string;
    point: [number, number];

    kind: string;
    slice?: SlicerRequest;
    inline?: InlineRequest;
};

type SourcePoint = [number, number];
type SourceRange = [SourcePoint, SourcePoint];

type SlicerResponse = {
    ranges_to_remove: SourceRange[];
};

type InlineResponse = {
    content: string;
};

function runSlicer(request: Request): Promise<SlicerResponse|InlineResponse> {
    return new Promise((resolve, reject) => {
        const proc = child_process.spawn(SLICER_BIN);
        proc.on('error', (err) => {
            reject(err);
        }).on('exit', (code) => {
            if (code !== 0) {
                reject(new Error(`Slicer exited with code ${code}`));
            }
        }).on('close', (code) => {
            if (code !== 0) {
                reject(new Error(`Slicer exited with code ${code}`));
            }
        });
        proc.stdout.on('data', (data) => {
            const response = JSON.parse(data.toString());
            resolve(response);
        });
        proc.stderr.on('data', (data) => {
            reject(new Error(data.toString()));
        });

        proc.stdin.write(JSON.stringify(request));
        proc.stdin.end();
    });
};

function removeRanges(src: String, ranges: SourceRange[], srcPoint: vscode.Position): [string, SourcePoint] {
    const lines = src.split('\n');
    let newContent = [];
    let newPoint: SourcePoint = [srcPoint.line, srcPoint.character];

    let i = 0;
    for (const [start, end] of ranges) {
        if (i < start[0]) {
            newContent.push(...lines.slice(i, start[0]));
        }
        const prefix = lines[start[0]].slice(0, start[1]).trim();
        if (prefix.length > 0) {
            newContent.push(prefix);
        }
        const suffix = lines[end[0]].slice(end[1]).trim();
        if (suffix.length > 0) {
            newContent.push(suffix);
        }
        i = end[0] + 1;

        if (end[0] < srcPoint.line) {
            let deletedLines = end[0] - start[0];
            if (prefix.length === 0 && suffix.length === 0) {
                deletedLines += 1;
            }
            newPoint[0] -= deletedLines;
        }
    }

    return [newContent.join('\n'), newPoint];
}

type DisplayHandler = (content: string, point: vscode.Position, language: string, resp: SlicerResponse) => void;

async function slice(direction: SliceDirection, displayFunc: DisplayHandler) {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        return;
    }

    const filename = editor.document.fileName;
    const content = editor.document.getText();
    const language = editor.document.languageId;
    const point = editor.selection.active;

    const req: Request = {
        filename,
        language,
        content,
        point: [point.line, point.character],
        kind: "slice",
        slice: {
            direction: direction,
        }
    };

    let resp: SlicerResponse;
    try {
        resp = await runSlicer(req) as SlicerResponse;
    } catch (e) {
        await vscode.window.showErrorMessage("Error slicing: " + (e as Error).toString());
        return;
    }

    await displayFunc(content, point, language, resp);
}

async function newDocDisplay(content: string, point: vscode.Position, language: string, resp: SlicerResponse) {
    const [filtered, targetPoint] = removeRanges(content, resp.ranges_to_remove, point);

    const sliceDoc = await vscode.workspace.openTextDocument({
        language: language,
        content: filtered,
    });

    await vscode.window.showTextDocument(sliceDoc, {
        preview: true,
        selection: new vscode.Range(targetPoint[0], targetPoint[1], targetPoint[0], targetPoint[1]),
        viewColumn: vscode.ViewColumn.Beside,
    });
}

async function foldDisplay(content: string, point: vscode.Position, language: string, resp: SlicerResponse) {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        return;
    }

    const ranges = resp.ranges_to_remove.map(([start, end]) => {
        return new vscode.Range(start[0], start[1], end[0], end[1]);
    });
    editor.selections = ranges.map((range) => {
        return new vscode.Selection(range.start, range.end);
    });
    await vscode.commands.executeCommand('editor.createFoldingRangeFromSelection');
}

async function inline() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        return;
    }

    const filename = editor.document.fileName;
    const content = editor.document.getText();
    const language = editor.document.languageId;
    const point = editor.selection.active;

    const targets = await vscode.commands.executeCommand<vscode.Location[]>('vscode.executeDefinitionProvider', editor.document.uri, point);
    if (targets.length === 0) {
        await vscode.window.showErrorMessage("Error inlining: No target definition(s) found");
        return;
    }

    // TODO: prompt user to select target
    const target = targets[0];

    const req: Request = {
        filename,
        language,
        content,
        point: [point.line, point.character],
        kind: "inline",
        inline: {
            target_content: (await vscode.workspace.openTextDocument(target.uri)).getText(),
            target_point: [target.range.start.line, target.range.start.character],
        }
    };
    console.log(JSON.stringify(req));

    let resp: InlineResponse;
    try {
        resp = await runSlicer(req) as InlineResponse;
    } catch (e) {
        console.log(e);
        await vscode.window.showErrorMessage("Error inlining: " + (e as Error).toString());
        return;
    }

    console.log(resp);

    let contentLines = content.split('\n');
    const inlineDoc = await vscode.workspace.openTextDocument({
        language: language,
        content: contentLines.slice(0, point.line).concat(resp.content).concat(contentLines.slice(point.line + 1)).join('\n'),
    });

    await vscode.window.showTextDocument(inlineDoc, {
        preview: true,
        selection: new vscode.Range(point.line, point.character, point.line, point.character),
        viewColumn: vscode.ViewColumn.Beside,
    });
}

export function activate(context: vscode.ExtensionContext) {
	context.subscriptions.push(vscode.commands.registerCommand('source-slicer.sliceBackwardW', async () => {
        await slice("Backward", newDocDisplay);
	}));
	context.subscriptions.push(vscode.commands.registerCommand('source-slicer.sliceForwardW', async () => {
        await slice("Forward", newDocDisplay);
	}));
	context.subscriptions.push(vscode.commands.registerCommand('source-slicer.sliceBackwardF', async () => {
        await slice("Backward", foldDisplay);
	}));
	context.subscriptions.push(vscode.commands.registerCommand('source-slicer.sliceForwardF', async () => {
        await slice("Forward", foldDisplay);
	}));

	context.subscriptions.push(vscode.commands.registerCommand('source-slicer.inline', async () => {
        await inline();
	}));
}

export function deactivate() {}