import * as vscode from 'vscode';
import * as child_process from 'child_process';

const SLICER_BIN = __dirname + "/../../../slicer/target/debug/slicer";

type SliceDirection = "Backward" | "Forward";

type SlicerRequest = {
    filename: string;
    content: string;
    point: [number, number];
    direction: SliceDirection;
};

type SlicerResponse = {
    content: string;
    point: [number, number];
};

function runSlicer(request: SlicerRequest): Promise<SlicerResponse> {
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

async function slice(direction: SliceDirection) {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        return;
    }

    const filename = editor.document.fileName;
    const content = editor.document.getText();
    const language = editor.document.languageId;
    const point = editor.selection.active;

    const req: SlicerRequest = {
        filename,
        content,
        point: [point.line, point.character],
        direction: direction,
    };

    let resp: SlicerResponse;
    try {
        resp = await runSlicer(req);
    } catch (e) {
        await vscode.window.showErrorMessage("Error slicing: " + (e as Error).toString());
        return;
    }

    const sliceDoc = await vscode.workspace.openTextDocument({
        language: language,
        content: resp.content,
    });
    // open the document as a preview
    await vscode.window.showTextDocument(sliceDoc, {
        preview: true,
        selection: new vscode.Range(resp.point[0], resp.point[1], resp.point[0], resp.point[1]),
        viewColumn: vscode.ViewColumn.Beside,
    });
}

export function activate(context: vscode.ExtensionContext) {
	context.subscriptions.push(vscode.commands.registerCommand('source-slicer.sliceBackward', async () => {
        await slice("Backward");
	}));
	context.subscriptions.push(vscode.commands.registerCommand('source-slicer.sliceForward', async () => {
        await slice("Forward");
	}));
}

export function deactivate() {}